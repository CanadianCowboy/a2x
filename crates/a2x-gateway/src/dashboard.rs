// a2x-gateway dashboard — live web UI for the A2X system
//
// Serves a single-page dashboard at `GET /` and streams live state
// via WebSocket at `/a2x/dashboard/ws`.

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use std::sync::Arc;
use std::time::SystemTime;

use a2x_core::graph::WorldGraph;
use a2x_core::memory::MemoryTrace;

use crate::listeners::http::HttpGatewayState;

pub async fn handle_dashboard() -> impl IntoResponse {
    axum::response::Html(DASHBOARD_HTML)
}

pub async fn handle_dashboard_ws(
    ws: WebSocketUpgrade,
    axum::extract::State(state): axum::extract::State<Arc<HttpGatewayState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| dashboard_ws_loop(socket, state))
}

async fn dashboard_ws_loop(mut socket: WebSocket, state: Arc<HttpGatewayState>) {
    let mut interval = tokio::time::interval(std::time::Duration::from_millis(500));
    let mut tick: u64 = 0;
    tick += 1;
    let snap = build_snapshot(&state, tick);
    if let Ok(snap) = snap {
        let json = serde_json::to_string(&snap).unwrap_or_default();
        if socket.send(Message::Text(json)).await.is_err() {
            return;
        }
    }
    loop {
        tokio::select! {
            _ = interval.tick() => {
                tick += 1;
                let snap = build_snapshot(&state, tick);
                if let Ok(snap) = snap {
                    let json = serde_json::to_string(&snap).unwrap_or_default();
                    if socket.send(Message::Text(json)).await.is_err() { break; }
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        let parsed: serde_json::Value = match serde_json::from_str(&text) {
                            Ok(v) => v,
                            Err(_) => {
                                let result = execute_dashboard_program(&state, &text);
                                let json = serde_json::json!({"type":"execute_result","result":result,"duration_ms":0}).to_string();
                                if socket.send(Message::Text(json)).await.is_err() { break; }
                                continue;
                            }
                        };
                        match parsed["type"].as_str() {
                            Some("chat") => { handle_chat_message(&mut socket, &state, parsed["message"].as_str().unwrap_or("")).await; }
                            Some("vm") => { handle_vm_command(&mut socket, &state, parsed["command"].as_str().unwrap_or("status"), &parsed).await; }
                            Some("models") => { handle_models_command(&mut socket, &state).await; }
                            Some("switch_model") => { handle_switch_model(&mut socket, &state, parsed["model"].as_str().unwrap_or("")).await; }
                            _ => {
                                let result = execute_dashboard_program(&state, &text);
                                let json = serde_json::json!({"type":"execute_result","result":result,"duration_ms":0}).to_string();
                                if socket.send(Message::Text(json)).await.is_err() { break; }
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) => break,
                    None => break,
                    _ => {}
                }
            }
        }
    }
}

// ── Model listing & switching ────────────────────────────────────────────

async fn handle_models_command(socket: &mut WebSocket, state: &Arc<HttpGatewayState>) {
    let api_url = state
        .gateway
        .lock()
        .ok()
        .map(|gw| {
            gw.config
                .chat_backend
                .api_url
                .trim_end_matches('/')
                .to_string()
        })
        .unwrap_or_default();
    // Parse base URL (scheme + host + port) instead of fragile ../ hack
    let tags_url = if let Some(end) = api_url.find("://") {
        let after_scheme = &api_url[end + 3..];
        if let Some(slash) = after_scheme.find('/') {
            format!("{}/api/tags", &api_url[..end + 3 + slash])
        } else {
            format!("{}/api/tags", api_url)
        }
    } else {
        format!("{}/api/tags", api_url)
    };
    match reqwest::get(&tags_url).await {
        Ok(resp) => match resp.json::<serde_json::Value>().await {
            Ok(json) => {
                let models: Vec<String> = json["models"].as_array().map_or(vec![], |arr| {
                    arr.iter()
                        .filter_map(|m| m["name"].as_str().map(|s| s.to_string()))
                        .collect()
                });
                let json = serde_json::json!({"type":"models_result","models":models}).to_string();
                let _ = socket.send(Message::Text(json)).await;
            }
            Err(e) => {
                let json = serde_json::json!({"type":"models_error","content":format!("Parse error: {}", e)}).to_string();
                let _ = socket.send(Message::Text(json)).await;
            }
        },
        Err(e) => {
            let json = serde_json::json!({"type":"models_error","content":format!("Ollama not reachable: {}", e)}).to_string();
            let _ = socket.send(Message::Text(json)).await;
        }
    }
}

async fn handle_switch_model(socket: &mut WebSocket, state: &Arc<HttpGatewayState>, model: &str) {
    if model.is_empty() {
        return;
    }
    let result = match state.gateway.lock() {
        Ok(mut gw) => {
            gw.switch_chat_model(model.to_string());
            gw.record_bus_event("system", &format!("Switched model to {}", model));
            serde_json::json!({"type":"switch_model_result","model":model,"success":true})
        }
        Err(e) => {
            serde_json::json!({"type":"switch_model_result","model":model,"success":false,"error":format!("Lock error: {}", e)})
        }
    };
    let _ = socket
        .send(Message::Text(
            serde_json::to_string(&result).unwrap_or_default(),
        ))
        .await;
}

// ── VM commands ────────────────────────────────────────────────────────────

async fn handle_vm_command(
    socket: &mut WebSocket,
    state: &Arc<HttpGatewayState>,
    command: &str,
    args: &serde_json::Value,
) {
    let vm_arc = match state
        .gateway
        .lock()
        .ok()
        .map(|gw| gw.chat_ccs_vm.clone())
    {
        Some(v) => v,
        None => {
            let _ = socket
                .send(Message::Text(
                    r#"{"type":"vm_error","content":"lock error"}"#.into(),
                ))
                .await;
            return;
        }
    };
    let result = match command {
        "status" => {
            let vm = vm_arc.lock().unwrap();
            serde_json::json!({"type":"vm_result","command":"status","data":{
                "graph_nodes":vm.world_graph.node_count(),"graph_edges":vm.world_graph.edge_count(),
                "memory_trace_length":vm.memory_trace.len(),"steps_executed":vm.steps_executed(),
                "uptime_secs":vm.uptime().as_secs_f32(),
                "program_id":vm.program().map(|p| p.id.to_string()),
                "regions":vm.region_names().iter().map(|(n,o,l)| serde_json::json!({"name":n,"offset":o,"length":l})).collect::<Vec<_>>()
            }})
        }
        "region" => {
            let vm = vm_arc.lock().unwrap();
            match vm.probe_region(args["region"].as_str().unwrap_or("belief")) {
                Some(a2x_ccs::ProbeSnapshot::Region {
                    name,
                    offset,
                    len,
                    data,
                }) => {
                    let stats = if data.len() >= 4 {
                        let sum: f32 = data.iter().sum();
                        let mean = sum / data.len() as f32;
                        let min = data.iter().fold(f32::MAX, |a, &b| a.min(b));
                        let max = data.iter().fold(f32::MIN, |a, &b| a.max(b));
                        Some(
                            serde_json::json!({"mean":format!("{:.4}",mean),"min":format!("{:.4}",min),"max":format!("{:.4}",max),"sum":format!("{:.4}",sum)}),
                        )
                    } else {
                        None
                    };
                    serde_json::json!({"type":"vm_result","command":"region","data":{"region":name,"offset":offset,"total_length":len,"preview":data.iter().take(32).copied().collect::<Vec<_>>(),"stats":stats}})
                }
                _ => serde_json::json!({"type":"vm_error","content":format!("Region not found")}),
            }
        }
        "query" => {
            let vm = vm_arc.lock().unwrap();
            let v = args["value"].as_str().unwrap_or("");
            if let Ok(Some(nid)) = vm.world_graph.lookup_label(v) {
                let node = vm.world_graph.lookup(nid).ok().flatten();
                serde_json::json!({"type":"vm_result","command":"query","data":{"query_type":args["query_type"].as_str().unwrap_or("label"),"node_id":nid.as_u64(),"label":node.as_ref().and_then(|n| n.label.clone()),"access_count":node.as_ref().map(|n| n.metadata.access_count)}})
            } else {
                serde_json::json!({"type":"vm_result","command":"query","data":{"found":false}})
            }
        }
        "trace" => {
            let vm = vm_arc.lock().unwrap();
            let snap = vm.probe_trace_tail(args["tail"].as_u64().unwrap_or(10) as usize);
            if let a2x_ccs::ProbeSnapshot::TraceSegment { entries } = snap {
                serde_json::json!({"type":"vm_result","command":"trace","data":{"count":entries.len(),"entries":entries.iter().map(|e| serde_json::json!({"ip":e.ip,"state_preview":e.state_preview.iter().take(4).copied().collect::<Vec<_>>()})).collect::<Vec<_>>()}})
            } else {
                serde_json::json!({"type":"vm_error","content":"Trace unavailable"})
            }
        }
        _ => {
            serde_json::json!({"type":"vm_error","content":format!("Unknown command: {}", command)})
        }
    };
    let _ = socket
        .send(Message::Text(
            serde_json::to_string(&result).unwrap_or_default(),
        ))
        .await;
}

// ── Chat ──────────────────────────────────────────────────────────────────

async fn handle_chat_message(
    socket: &mut WebSocket,
    state: &Arc<HttpGatewayState>,
    user_message: &str,
) {
    let gw_arc = state.gateway.clone();
    let chat_agent = gw_arc.lock().ok().map(|mut gw| gw.get_chat_agent());
    let chat_agent = match chat_agent {
        Some(a) => a,
        None => {
            let _ = socket
                .send(Message::Text(
                    r#"{"type":"chat_error","content":"lock error"}"#.into(),
                ))
                .await;
            return;
        }
    };
    if let Ok(mut gw) = gw_arc.lock() {
        gw.record_bus_event(
            "chat",
            &format!("user: {}", &user_message[..user_message.len().min(60)]),
        );
    }
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let um = user_message.to_string();
    let agent_for_save = chat_agent.clone();
    let stream_task = tokio::spawn(async move {
        chat_agent
            .chat_streaming(&um, &move |c| {
                let _ = tx.send(serde_json::to_string(&c).unwrap_or_default());
            })
            .await
    });
    let mut full_text = String::new();
    loop {
        match rx.recv().await {
            Some(js) => {
                let chunk: serde_json::Value = serde_json::from_str(&js).unwrap_or_default();
                match chunk["type"].as_str().unwrap_or("") {
                    "text" => {
                        if let Some(c) = chunk["content"].as_str() {
                            full_text.push_str(c);
                        }
                        if socket.send(Message::Text(js)).await.is_err() {
                            return;
                        }
                    }
                    "tool_call" | "tool_call_done" | "tool_result" => {
                        if socket.send(Message::Text(js)).await.is_err() {
                            return;
                        }
                    }
                    "stream_done" | "done" => {
                        let _ = socket.send(Message::Text(serde_json::json!({"type":"chat_done","total_tokens":chunk["total_tokens"]}).to_string())).await;
                        break;
                    }
                    "warning" | "error" => {
                        if socket.send(Message::Text(js)).await.is_err() {
                            return;
                        }
                        break;
                    }
                    _ => {
                        if socket.send(Message::Text(js)).await.is_err() {
                            return;
                        }
                    }
                }
            }
            None => {
                let _ = socket
                    .send(Message::Text(r#"{"type":"chat_done"}"#.into()))
                    .await;
                break;
            }
        }
    }
    match stream_task.await {
        Ok(Ok(())) => {
            if let Ok(mut gw) = gw_arc.lock() {
                gw.record_bus_event(
                    "chat",
                    &format!("assistant: {}", &full_text[..full_text.len().min(60)]),
                );
            }
            if let Some(path) = crate::gateway::GatewayState::conversation_path() {
                let _ = agent_for_save.save_conversation(&path);
            }
        }
        Ok(Err(e)) => {
            let _ = socket
                .send(Message::Text(
                    serde_json::json!({"type":"chat_error","content":format!("{}",e)}).to_string(),
                ))
                .await;
        }
        Err(e) => {
            let _ = socket
                .send(Message::Text(
                    serde_json::json!({"type":"chat_error","content":format!("Task: {}",e)})
                        .to_string(),
                ))
                .await;
        }
    }
}

// ── Snapshot ──────────────────────────────────────────────────────────────

fn build_snapshot(state: &Arc<HttpGatewayState>, tick: u64) -> Result<serde_json::Value, String> {
    let gw = state.gateway.lock().map_err(|e| format!("lock: {}", e))?;
    let mut entities: Vec<serde_json::Value> = gw.list_entities().iter().map(|e| serde_json::json!({
        "id":e.id.to_string(),"entity_type":format!("{:?}",e.entity_type),"display_name":e.display_name,
        "capabilities":e.capabilities.iter().map(|c| c.to_string()).collect::<Vec<_>>()
    })).collect();
    let bus_agents: Vec<serde_json::Value> = gw.bus.lock().map_or(vec![], |b| {
        b.discover(&a2x_bus::AgentFilter::All).iter().map(|info| serde_json::json!({
        "id":info.id.as_str(),"entity_type":format!("{:?}",info.agent_type),
        "display_name":format!("{} ({:?})",info.id.as_str(),info.agent_type),
        "capabilities":info.capabilities.iter().map(|c| c.to_string()).collect::<Vec<_>>()
    })).collect()
    });
    entities.extend(bus_agents);
    let agent_count = gw.bus.lock().map_or(0, |b| b.agent_count());

    // ── Single VM lock for consistent snapshot ──
    let (graph_nodes, graph_edges, vm_status, vm_regions) = {
        let vm = gw.chat_ccs_vm.lock().map_err(|e| format!("vm: {}", e))?;
        let node_ids = vm.world_graph.node_ids();
        let (gn, ge) = if node_ids.is_empty() {
            (vec![], vec![])
        } else {
            let nodes: Vec<serde_json::Value> = node_ids.iter().take(40).filter_map(|nid| {
                vm.world_graph.lookup(*nid).ok().flatten().map(|node| {
                    let val = node.concept.data.first().copied().unwrap_or(0.5).abs().min(1.0);
                    serde_json::json!({"id": format!("n{}", node.id.as_u64()), "label": node.label.unwrap_or_else(|| format!("n{}", node.id.as_u64())), "val": val})
                })
            }).collect();
            let edges: Vec<serde_json::Value> = node_ids.iter().take(20).flat_map(|nid| {
                vm.world_graph.neighbors(*nid).unwrap_or_default().into_iter().map(move |tgt|
                    serde_json::json!({"from": format!("n{}", nid.as_u64()), "to": format!("n{}", tgt.as_u64())})
                )
            }).collect();
            (nodes, edges)
        };
        let vs = serde_json::json!({"graph_nodes":vm.world_graph.node_count(),"graph_edges":vm.world_graph.edge_count(),"trace_len":vm.memory_trace.len(),"steps":vm.steps_executed(),"uptime_secs":vm.uptime().as_secs_f32()});
        let vr: Vec<serde_json::Value> = vm
            .region_names()
            .iter()
            .map(|(name, _off, len)| {
                let data = vm
                    .probe_region(name)
                    .map(|s| {
                        if let a2x_ccs::ProbeSnapshot::Region { data, .. } = s {
                            data.iter().take(64).copied().collect::<Vec<f32>>()
                        } else {
                            vec![]
                        }
                    })
                    .unwrap_or_default();
                serde_json::json!({"name":name,"length":len,"preview":data})
            })
            .collect();
        (gn, ge, vs, vr)
    };

    let bus_events: Vec<serde_json::Value> = gw
        .clone_bus_log()
        .iter()
        .map(|ev| serde_json::json!({"ts":ev.timestamp,"type":ev.event_type,"msg":ev.message}))
        .collect();
    let history: Vec<serde_json::Value> = gw.clone_program_history().iter().map(|h| serde_json::json!({"ts":h.timestamp,"source":h.source,"result":h.result,"status":h.status,"duration_ms":h.duration_ms})).collect();
    let heatmap_data: Vec<f32> = (0..64)
        .map(|i| ((i as f32 / 64.0) * std::f32::consts::PI * 2.0).sin() * 0.5 + 0.5)
        .collect();
    let chat_context = if let Some(ref chat) = gw.chat_agent {
        let hist = chat.history();
        serde_json::json!({"used":hist.iter().map(|m| (m.content.chars().count()as f64/4.0).ceil()as u32+8).sum::<u32>(),"max":chat.max_context_tokens(),"history_msgs":hist.len()})
    } else {
        serde_json::json!({"used":0,"max":32768,"history_msgs":0})
    };
    Ok(
        serde_json::json!({"type":"snapshot","tick":tick,"agent_count":agent_count,"entity_count":entities.len(),"entities":entities,"bus_events":bus_events,"bus_log_idx":bus_events.len() as u64,"history":history,"hist_idx":history.len() as u64,"world_graph":{"nodes":graph_nodes,"edges":graph_edges},"heatmap":heatmap_data,"heatmap_width":8,"chat_context":chat_context,"vm_status":vm_status,"vm_regions":vm_regions}),
    )
}

fn execute_dashboard_program(state: &Arc<HttpGatewayState>, source: &str) -> String {
    let program = match a2x_sigma::parse_program(source) {
        Ok(p) => p,
        Err(e) => {
            let msg = format!("Parse: {}", e);
            if let Ok(mut gw) = state.gateway.lock() {
                gw.record_execution(source, &msg, "parse_error", 0);
            }
            return msg;
        }
    };
    let start = SystemTime::now();
    let result = match state.gateway.lock() {
        Ok(gw) => match gw.execute_program(&program) {
            Ok(p) => p,
            Err(e) => {
                let msg = format!("Execute: {}", e);
                drop(gw);
                if let Ok(mut gw) = state.gateway.lock() {
                    gw.record_execution(source, &msg, "error", 0);
                }
                return msg;
            }
        },
        Err(e) => return format!("Lock: {}", e),
    };
    let duration_ms = start.elapsed().map(|d| d.as_millis() as u64).unwrap_or(0);
    let output = if result.is_empty() {
        "∅ (empty result)".to_string()
    } else {
        result
            .instructions
            .iter()
            .map(a2x_sigma::serialize_packet)
            .collect::<Vec<_>>()
            .join("\n")
    };
    if let Ok(mut gw) = state.gateway.lock() {
        gw.record_execution(
            &source.chars().take(80).collect::<String>(),
            &output.chars().take(80).collect::<String>(),
            "completed",
            duration_ms,
        );
    }
    output
}

// ── Dashboard HTML ────────────────────────────────────────────────────────

const DASHBOARD_HTML: &str = r#"<!DOCTYPE html>
<html lang="en" data-theme="dark">
<head>
<meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>A2X Dashboard</title>
<style>
:root,[data-theme=dark]{--bg:#08080f;--surface:#10101a;--surface2:#181828;--border:#282840;--text:#c8c8d8;--text-dim:#686878;--cyan:#00e5ff;--magenta:#ff4081;--green:#69f0ae;--yellow:#ffd740;--red:#ff5252;--font:'SF Mono','Fira Code','Consolas',monospace}
[data-theme=light]{--bg:#f0f0f5;--surface:#fff;--surface2:#eeeef4;--border:#d0d0e0;--text:#1a1a2e;--text-dim:#787898;--cyan:#0077b6;--magenta:#c9184a;--green:#2d6a4f;--yellow:#e09f00;--red:#d00000}
*{margin:0;padding:0;box-sizing:border-box}body{background:var(--bg);color:var(--text);font-family:var(--font);font-size:12px;overflow:hidden;height:100vh;display:flex;flex-direction:column}
.h{background:var(--surface);border-bottom:1px solid var(--border);padding:6px 14px;display:flex;align-items:center;gap:14px;flex-shrink:0}.h h1{font-size:15px;font-weight:700;color:var(--cyan);letter-spacing:2px}
.h .st{display:flex;align-items:center;gap:6px;font-size:10px;color:var(--text-dim)}.h .dot{width:7px;height:7px;border-radius:50%;background:var(--green);animation:pulse 2s infinite}
.h .btns{display:flex;gap:6px;margin-left:auto}.h .btns button{background:var(--surface2);color:var(--text-dim);border:1px solid var(--border);padding:4px 10px;font-family:var(--font);font-size:10px;cursor:pointer;border-radius:3px}.h .btns button:hover{color:var(--cyan);border-color:var(--cyan)}
@keyframes pulse{0%,100%{opacity:1}50%{opacity:.25}}.main{display:flex;flex:1;overflow:hidden}
.p{border-right:1px solid var(--border);background:var(--surface);overflow-y:auto}.pl{width:260px;flex-shrink:0}.pc{flex:1;display:flex;flex-direction:column}.pr{width:380px;flex-shrink:0;border-left:1px solid var(--border);display:flex;flex-direction:column}
.ph{padding:7px 10px;background:var(--surface2);border-bottom:1px solid var(--border);font-size:10px;text-transform:uppercase;letter-spacing:1.5px;color:var(--text-dim);position:sticky;top:0;z-index:2}
.model-sel{background:var(--surface2);color:var(--text);border:1px solid var(--border);padding:2px 6px;font-family:var(--font);font-size:9px;border-radius:2px;outline:none;margin-left:6px}.model-sel:hover{border-color:var(--cyan)}
.card{padding:8px 10px;border-bottom:1px solid var(--border)}.card:hover{background:var(--surface2)}.card .nm{font-size:12px;color:var(--text);margin-bottom:3px;display:flex;align-items:center;gap:6px}.card .badge{font-size:9px;padding:1px 5px;border-radius:2px;background:var(--surface2);border:1px solid var(--border);color:var(--text-dim)}.card .caps{display:flex;flex-wrap:wrap;gap:3px;margin-top:3px}.card .cap{font-size:8px;padding:1px 4px;border-radius:2px;background:rgba(0,229,255,.1);color:var(--cyan);border:1px solid rgba(0,229,255,.15)}
#gc{width:100%;height:100%;position:relative;cursor:grab}#gcv{width:100%;height:100%;display:block}
.tooltip{position:absolute;pointer-events:none;background:var(--surface);border:1px solid var(--cyan);border-radius:4px;padding:8px 10px;font-size:10px;z-index:10;display:none;box-shadow:0 4px 16px rgba(0,0,0,.4);max-width:200px;white-space:nowrap}
.log{font-size:10px;line-height:1.5;padding:6px 10px;max-height:160px;overflow-y:auto;border-top:1px solid var(--border);background:var(--bg)}.log .e{padding:1px 0;display:flex;gap:8px;animation:fadeIn .3s}
@keyframes fadeIn{from{opacity:0;transform:translateY(-4px)}to{opacity:1;transform:translateY(0)}}
.tabs{display:flex;border-bottom:1px solid var(--border)}.tabs button{flex:1;padding:6px 10px;background:var(--surface);color:var(--text-dim);border:none;font-family:var(--font);font-size:10px;cursor:pointer;letter-spacing:1px}.tabs button.active{color:var(--cyan);background:var(--surface2);border-bottom:2px solid var(--cyan)}
.tab-content{display:none;flex:1;overflow:hidden;flex-direction:column}.tab-content.active{display:flex}
.play textarea{flex:1;background:var(--bg);color:var(--cyan);border:1px solid var(--border);border-radius:4px;padding:8px;font-family:var(--font);font-size:12px;resize:none;outline:none}.play .btn-row{display:flex;gap:6px}.play .btn-row button{flex:1;background:var(--cyan);color:#000;border:none;padding:7px 14px;font-family:var(--font);font-size:11px;font-weight:700;border-radius:4px;cursor:pointer}.play .btn-row button.secondary{background:var(--surface2);color:var(--text);border:1px solid var(--border)}.play .res{flex:1;background:var(--bg);border:1px solid var(--border);border-radius:4px;padding:8px;font-size:11px;overflow-y:auto;color:var(--green);white-space:pre-wrap;word-break:break-all;min-height:0}
.chat-msgs{flex:1;overflow-y:auto;padding:8px;display:flex;flex-direction:column;gap:6px}.chat-msg{max-width:90%;padding:8px 10px;border-radius:8px;font-size:11px;line-height:1.5;word-break:break-word;animation:fadeIn .3s}.chat-msg.user{align-self:flex-end;background:rgba(0,229,255,.12);border:1px solid rgba(0,229,255,.2);border-bottom-right-radius:2px}.chat-msg.assistant{align-self:flex-start;background:var(--surface2);border:1px solid var(--border);border-bottom-left-radius:2px}.chat-msg .role{font-size:9px;text-transform:uppercase;letter-spacing:1px;margin-bottom:4px;color:var(--cyan)}.chat-msg .role.ast{color:var(--green)}.chat-msg pre{background:var(--bg);border:1px solid var(--border);border-radius:3px;padding:6px 8px;margin:4px 0;font-size:10px;overflow-x:auto}.chat-msg code{font-family:var(--font);color:var(--cyan)}
.chat-input{display:flex;padding:8px;gap:6px;border-top:1px solid var(--border);background:var(--surface)}.chat-input input{flex:1;background:var(--bg);color:var(--text);border:1px solid var(--border);border-radius:4px;padding:8px;font-family:var(--font);font-size:12px;outline:none}.chat-input button{background:var(--cyan);color:#000;border:none;padding:6px 12px;font-family:var(--font);font-size:11px;font-weight:700;border-radius:4px;cursor:pointer}
.chat-indicator{font-size:10px;color:var(--text-dim);padding:4px 10px;display:flex;align-items:center;gap:6px}.chat-msg .tool-indicator{font-size:9px;color:var(--text-dim);padding:3px 0;margin:2px 0;border-top:1px solid var(--border);animation:fadeIn .3s}.chat-msg .tool-indicator.ok{color:var(--green)}.chat-msg .tool-indicator.err{color:var(--red)}
.ctx-bar{display:flex;align-items:center;gap:6px;flex-shrink:0;min-width:120px}.ctx-bar .ctx-label{font-size:9px;color:var(--text-dim);white-space:nowrap}.ctx-bar .ctx-track{flex:1;height:3px;background:var(--surface2);border-radius:2px;overflow:hidden}.ctx-bar .ctx-fill{height:100%;border-radius:2px;transition:width .8s ease,background .8s ease}@keyframes spin{to{transform:rotate(360deg)}}
.history{max-height:160px;overflow-y:auto;border-top:1px solid var(--border)}.history .he{padding:4px 10px;border-bottom:1px solid var(--border);font-size:10px;display:flex;gap:8px;animation:fadeIn .3s}.history .he .hs{font-size:9px;padding:1px 5px;border-radius:2px;flex-shrink:0}.history .he .hs.ok{background:rgba(105,240,174,.15);color:var(--green);border:1px solid rgba(105,240,174,.2)}
.heatmap{display:grid;gap:1px;padding:6px;max-height:140px;overflow:hidden}.heatmap .cell{aspect-ratio:1;border-radius:1px}.vm-heatmap{display:flex;flex-wrap:wrap;gap:1px;padding:6px;max-height:60px;overflow:hidden}.vm-heatmap .vcell{width:8px;height:8px;border-radius:1px}
.vm-panel{padding:4px 10px;font-size:10px;color:var(--text-dim);border-top:1px solid var(--border);background:var(--bg);flex-shrink:0;max-height:100px;overflow-y:auto}.vm-panel .vm-btn{background:var(--surface2);color:var(--text-dim);border:1px solid var(--border);padding:1px 6px;font-family:var(--font);font-size:8px;cursor:pointer;border-radius:2px}.vm-panel .vm-btn:hover{color:var(--cyan);border-color:var(--cyan)}
.sbar{padding:6px 10px;border-top:1px solid var(--border);display:flex;gap:12px;font-size:9px;color:var(--text-dim);background:var(--surface2);flex-shrink:0}.sbar .sv{color:var(--cyan);font-weight:700}.shadow{position:fixed;inset:0;background:rgba(0,0,0,.6);z-index:50;display:none;align-items:center;justify-content:center}.shadow.open{display:flex}.shortcuts{background:var(--surface);border:1px solid var(--border);border-radius:8px;padding:20px 24px;max-width:400px}.shortcuts h2{color:var(--cyan);font-size:13px;margin-bottom:12px}.shortcuts .row{display:flex;justify-content:space-between;padding:4px 0;font-size:11px;border-bottom:1px solid var(--border)}.shortcuts .kbd{background:var(--surface2);padding:1px 6px;border-radius:3px;border:1px solid var(--border);font-size:10px;color:var(--cyan)}.ph .lbtn{background:var(--surface);color:var(--text-dim);border:1px solid var(--border);padding:2px 7px;font-family:var(--font);font-size:9px;cursor:pointer;border-radius:2px;margin-left:4px}.ph .lbtn.on{color:var(--cyan);border-color:var(--cyan);background:rgba(0,229,255,.1)}.timeline{display:flex;align-items:flex-end;gap:1px;height:20px;padding:0 2px}.timeline .bar{border-radius:1px 1px 0 0;min-width:3px}
</style></head><body>
<div class="h"><h1>A2X</h1><div class="st"><span class="dot" id="dot"></span><span id="conn-label">connecting</span></div><div class="btns"><button onclick="toggleTheme()">☀</button><button onclick="openShortcuts()">?</button></div></div>
<div class="main">
<div class="p pl"><div class="ph">Agents</div><div id="agents"><div class="card"><div class="nm">No agents connected</div></div></div></div>
<div class="p pc">
<div class="ph">WorldGraph <span style="font-size:9px;color:var(--text-dim);margin-left:8px">(drag, scroll, dbl-click)</span><button class="lbtn on" onclick="setLayout('force')" id="lb-force">Force</button><button class="lbtn" onclick="setLayout('circular')" id="lb-circ">Circular</button><button class="lbtn" onclick="setLayout('grid')" id="lb-grid">Grid</button></div>
<div id="gc"><canvas id="gcv"></canvas><div class="tooltip" id="tt"></div></div>
<div class="ph">StateField Heatmap</div><div class="heatmap" id="heatmap"></div>
<div class="ph">VM Regions</div><div class="vm-heatmap" id="vm-region-heatmap"></div>
<div class="vm-panel" id="vm-panel"><button class="vm-btn" onclick="vmCmd('status')">Status</button> <button class="vm-btn" onclick="vmCmd('region','belief')">Belief</button> <button class="vm-btn" onclick="vmCmd('region','attention')">Attention</button> <button class="vm-btn" onclick="vmCmd('region','goal')">Goal</button> <button class="vm-btn" onclick="vmCmd('trace')">Trace</button> <span id="vm-info" style="margin-left:8px">nodes:0 edges:0 steps:0</span></div>
<div class="ph">Bus Traffic</div><div class="log" id="blog"></div>
</div>
<div class="p pr">
<div class="tabs"><button class="active" onclick="switchTab('sigma')" id="tab-sigma">Sigma</button><button onclick="switchTab('chat')" id="tab-chat">Chat</button></div>
<div class="tab-content active" id="tab-content-sigma"><div class="play">
<div class="ed" style="flex:1;padding:10px;display:flex;flex-direction:column;gap:6px;min-height:0"><textarea id="si" placeholder="Sigma program..." rows="3">I:⚡✣  C:⟨sys⟩  P:⥂  D:⌵</textarea><div class="btn-row"><button onclick="execute()">Execute</button><button class="secondary" onclick="resetPlayground()">Clear</button></div></div>
<div class="ph">Result</div><div class="res" id="sr">Ready</div></div><div class="history" id="hist"></div></div>
<div class="tab-content" id="tab-content-chat">
<div class="ph" style="position:static;display:flex;align-items:center;gap:8px"><span>Chat</span><select class="model-sel" id="model-sel" onchange="switchModel(this.value)"><option value="">loading...</option></select><div class="ctx-bar" id="ctx-bar"><span class="ctx-label" id="ctx-label">0/32K</span><div class="ctx-track"><div class="ctx-fill" id="ctx-fill" style="width:0%;background:var(--green)"></div></div></div></div>
<div class="chat-msgs" id="chat-msgs"><div class="chat-msg assistant"><div class="role ast">Assistant</div>Welcome! I'm the A2X Chat Agent.</div></div>
<div class="chat-input"><input id="chat-input" placeholder="Ask the A2X agent..." onkeydown="chatKeydown(event)"><button id="chat-send" onclick="sendChat()">Send</button></div>
<div class="chat-indicator" id="chat-indicator" style="display:none">Thinking...</div>
</div></div></div>
<div class="sbar"><span>Agents:<span class="sv" id="sa">0</span></span><span>Entities:<span class="sv" id="se">0</span></span><span>Nodes:<span class="sv" id="sn">0</span></span><span>Edges:<span class="sv" id="sne">0</span></span><span>Execs:<span class="sv" id="sx">0</span></span><span>Tick:<span class="sv" id="stk">0</span></span><span style="margin-left:auto">Timeline:</span><div class="timeline" id="timeline"></div></div>
<div class="shadow" id="shadow" onclick="closeShortcuts()"><div class="shortcuts" onclick="event.stopPropagation()"><h2>Shortcuts</h2><div class="row"><span>Execute</span><span class="kbd">Ctrl+Enter</span></div><div class="row"><span>Theme</span><span class="kbd">Ctrl+T</span></div><div class="row"><span>Reset</span><span class="kbd">0</span></div><div class="row"><span>Help</span><span class="kbd">?</span></div></div></div>
<script>
let execCount=0,graphNodes=[],graphEdges=[],heatData=[],heatW=8,panX=0,panY=0,zoom=1,panning=false,panSX=0,panSY=0,ws,reconnectDelay=500,wsUrl,chatting=false;const MAX_RECONNECT=30000;

function connectWS(){wsUrl=wsUrl||((location.protocol==='https:'?'wss:':'ws:')+'//'+location.host+'/a2x/dashboard/ws');ws=new WebSocket(wsUrl);ws.onopen=()=>{document.getElementById('dot').style.background='var(--green)';document.getElementById('conn-label').textContent='connected';reconnectDelay=500;fetchModels();};ws.onclose=()=>{document.getElementById('dot').style.background='var(--red)';document.getElementById('conn-label').textContent='reconnecting';setTimeout(connectWS,reconnectDelay);reconnectDelay=Math.min(reconnectDelay*2,MAX_RECONNECT);};
ws.onmessage=(e)=>{const d=JSON.parse(e.data);if(d.type==='snapshot')updateAll(d);else if(d.type==='execute_result'){showResult(d.result||'empty');addTimeline('ok',d.duration_ms||5);}else if(d.type==='chat_error')handleChatError(d.content||'Error');else if(d.type==='text')handleChatStream(d.content||'');else if(d.type==='tool_call'||d.type==='tool_call_done'){addLog('chat','Tool: '+(d.tool||''));if(streamEl){let ind=document.createElement('div');ind.className='tool-indicator';ind.textContent='Running: '+(d.tool||'');streamEl.appendChild(ind);}}else if(d.type==='tool_result'){addLog(d.success?'chat':'exec',(d.success?'✓':'✗')+' '+(d.tool||'')+': '+(d.content_preview||'').substring(0,60));if(streamEl){let ind=document.createElement('div');ind.className='tool-indicator '+(d.success?'ok':'err');ind.textContent=(d.success?'✓':'✗')+' '+(d.tool||'');streamEl.appendChild(ind);}}else if(d.type==='chat_done')handleChatDone(d.total_tokens||0);else if(d.type==='vm_result')handleVmResult(d);else if(d.type==='vm_error')addLog('system','VM: '+d.content);else if(d.type==='models_result'){let sel=document.getElementById('model-sel');sel.innerHTML=d.models.map(m=>'<option value="'+m+'">'+m+'</option>').join('');}else if(d.type==='switch_model_result'){addLog('system','Model: '+(d.success?'✓ Switched to ':'✗ Failed: ')+d.model);}};}
connectWS();

function fetchModels(){ws.send(JSON.stringify({type:'models'}));}
function switchModel(m){if(m)ws.send(JSON.stringify({type:'switch_model',model:m}));}
function vmCmd(cmd,region){let m={type:'vm',command:cmd};if(region)m.region=region;ws.send(JSON.stringify(m));}
function handleVmResult(d){if(d.command==='status'){let s=d.data;document.getElementById('vm-info').textContent='nodes:'+s.graph_nodes+' edges:'+s.graph_edges+' steps:'+s.steps_executed;addLog('system','VM: '+s.graph_nodes+'n '+s.graph_edges+'e '+s.steps_executed+'s');}else if(d.command==='region'&&d.data&&d.data.preview){let vals=d.data.preview,max=Math.max(...vals.map(Math.abs));let h='';for(let i=0;i<vals.length;i++){let v=vals[i],a=Math.abs(v)/Math.max(max,0.01);h+='<div class="vcell" style="background:rgb('+Math.floor(a*255)+','+Math.floor((1-a)*80)+','+Math.floor(a*200)+')" title="'+v.toFixed(4)+'"></div>';}document.getElementById('vm-region-heatmap').innerHTML=h;addLog('system','VM '+d.data.region+': '+d.data.total_length+'v');}else if(d.command==='query'){addLog('system','VM query: '+(d.data.found?'node '+d.data.node_id:'not found'));}else if(d.command==='trace'){addLog('system','VM trace: '+d.data.count+' entries');}}

function switchTab(tab){document.querySelectorAll('.tabs button').forEach(b=>b.classList.remove('active'));document.querySelectorAll('.tab-content').forEach(c=>c.classList.remove('active'));document.getElementById('tab-'+tab).classList.add('active');document.getElementById('tab-content-'+tab).classList.add('active');if(tab==='chat')document.getElementById('chat-input').focus();else document.getElementById('si').focus();}
function sendChat(){if(chatting)return;const inp=document.getElementById('chat-input');const msg=inp.value.trim();if(!msg)return;inp.value='';addChatMessage('user',msg);setChatting(true);ws.send(JSON.stringify({type:'chat',message:msg}));}
function chatKeydown(e){if(e.key==='Enter'&&!e.shiftKey){e.preventDefault();sendChat();}}
let streamEl=null,streamBuf='',streamDirty=false,streamRafId=null;
function handleChatStream(text){if(!streamEl){streamEl=document.createElement('div');streamEl.className='chat-msg assistant';streamEl.innerHTML='<div class="role ast">Assistant</div><div class="stream-content"></div>';document.getElementById('chat-msgs').appendChild(streamEl);}streamBuf+=text;if(!streamDirty){streamDirty=true;streamRafId=requestAnimationFrame(()=>{streamDirty=false;if(streamEl){streamEl.querySelector('.stream-content').innerHTML=formatChatContent(streamBuf);document.getElementById('chat-msgs').scrollTop=document.getElementById('chat-msgs').scrollHeight;}});}}
function handleChatDone(tokens){if(streamEl){if(streamRafId)cancelAnimationFrame(streamRafId);streamEl.innerHTML='<div class="role ast">Assistant</div>'+formatChatContent(streamBuf);}streamEl=null;streamBuf='';streamDirty=false;setChatting(false);addLog('chat','Done'+(tokens?' ('+tokens+'t)':''));}
function handleChatError(text){if(streamEl){if(streamRafId)cancelAnimationFrame(streamRafId);streamEl.remove();streamEl=null;streamBuf='';streamDirty=false;}addChatMessage('assistant','Error: '+text);setChatting(false);}
function updateContextBar(ctx){const pct=ctx.max>0?Math.min(100,(ctx.used/ctx.max)*100):0;const fill=document.getElementById('ctx-fill'),label=document.getElementById('ctx-label');fill.style.width=pct+'%';fill.style.background=pct>85?'var(--red)':pct>65?'var(--yellow)':'var(--green)';label.textContent=fmtK(ctx.used)+'/'+fmtK(ctx.max)+' ('+ctx.history_msgs+'m)';}
function fmtK(n){return n>=1000?(n/1000).toFixed(1)+'K':n.toString();}
function setChatting(v){chatting=v;document.getElementById('chat-send').disabled=v;document.getElementById('chat-input').disabled=v;document.getElementById('chat-indicator').style.display=v?'flex':'none';}
function addChatMessage(role,content){const el=document.getElementById('chat-msgs'),div=document.createElement('div');div.className='chat-msg '+role;div.innerHTML='<div class="role '+(role==='assistant'?'ast':'')+'">'+(role==='user'?'You':'Assistant')+'</div>'+formatChatContent(content);el.appendChild(div);el.scrollTop=el.scrollHeight;}
function formatChatContent(text){let out='',inCode=false,codeBuf='';const lines=text.split('\n');for(let i=0;i<lines.length;i++){const line=lines[i];if(line.trim().startsWith('```')){if(inCode){out+='<pre><code>'+esc(codeBuf.trim())+'</code></pre>';codeBuf='';inCode=false;}else{inCode=true;}}else if(inCode){codeBuf+=line+'\n';}else{out+=line.replace(/`([^`]+)`/g,'<code>$1</code>')+'\n';}}if(inCode)out+='<pre><code>'+esc(codeBuf.trim())+'</code></pre>';return out||esc(text);}
function updateAll(s){document.getElementById('sa').textContent=s.agent_count;document.getElementById('se').textContent=s.entity_count;document.getElementById('sn').textContent=(s.world_graph?.nodes||[]).length;document.getElementById('sne').textContent=(s.world_graph?.edges||[]).length;document.getElementById('stk').textContent=s.tick;if(s.chat_context)updateContextBar(s.chat_context);if(s.world_graph?.nodes?.length>0&&graphNodes.length===0){graphNodes=s.world_graph.nodes.map(n=>({id:n.id,label:n.label||n.id,val:n.val||0.5,x:0,y:0,vx:0,vy:0}));graphEdges=(s.world_graph.edges||[]).map(e=>({from:e.from,to:e.to}));initGraph();}(s.bus_events||[]).forEach(ev=>addLog(ev.type,ev.msg));(s.history||[]).forEach(h=>addHistory(h));if(s.heatmap){heatData=s.heatmap;heatW=s.heatmap_width||8;drawHeatmap();}if(s.vm_status){document.getElementById('vm-info').textContent='nodes:'+s.vm_status.graph_nodes+' edges:'+s.vm_status.graph_edges+' steps:'+s.vm_status.steps;}if(s.vm_regions&&s.vm_regions.length>0){let r=s.vm_regions[0];if(r.preview&&r.preview.length>0){let vals=r.preview,max=Math.max(...vals.map(Math.abs));let h='';for(let i=0;i<vals.length;i++){let v=vals[i],a=Math.abs(v)/Math.max(max,0.01);h+='<div class="vcell" style="background:rgb('+Math.floor(a*255)+','+Math.floor((1-a)*80)+','+Math.floor(a*200)+')" title="'+r.name+'['+i+']='+v.toFixed(4)+'"></div>';}document.getElementById('vm-region-heatmap').innerHTML=h;}}updateAgents(s.entities||[]);}
function updateAgents(ents){const el=document.getElementById('agents');if(!ents.length){el.innerHTML='<div class="card"><div class="nm">No agents connected</div></div>';return;}el.innerHTML=ents.map(e=>'<div class="card"><div class="nm"><span style="color:var(--green)">●</span>'+esc(e.display_name||e.id)+'<span class="badge">'+esc(e.entity_type)+'</span></div><div class="caps">'+(e.capabilities||[]).map(c=>'<span class="cap">'+esc(c)+'</span>').join('')+'</div></div>').join('');}
function addLog(type,msg){const log=document.getElementById('blog'),time=new Date().toLocaleTimeString(),color=type==='bus'?'var(--cyan)':type==='exec'?'var(--green)':type==='chat'?'var(--magenta)':type==='system'?'var(--yellow)':'var(--text-dim)';log.innerHTML+='<div class="e"><span style="color:'+color+'">'+time+'</span><span>'+esc(msg)+'</span></div>';log.scrollTop=log.scrollHeight;while(log.children.length>200)log.removeChild(log.firstChild);}
function addHistory(h){const hist=document.getElementById('hist'),cls=h.status==='completed'?'ok':'err';hist.innerHTML+='<div class="he"><span class="hs '+cls+'">'+(h.status==='completed'?'OK':'ERR')+'</span><span>'+esc((h.source||'').substring(0,50))+'</span><span>'+(h.duration_ms?h.duration_ms+'ms':'')+'</span></div>';hist.scrollTop=hist.scrollHeight;while(hist.children.length>50)hist.removeChild(hist.firstChild);}
function drawHeatmap(){const el=document.getElementById('heatmap');el.style.gridTemplateColumns='repeat('+heatW+',1fr)';el.innerHTML=heatData.map(v=>{let r=Math.floor(v*255),g=Math.floor((1-v)*100),b=Math.floor(Math.abs(v-.5)*400);return '<div class="cell" style="background:rgb('+Math.min(255,Math.max(0,r))+','+Math.min(255,Math.max(0,g))+','+Math.min(255,Math.max(0,b))+')" title="'+v.toFixed(3)+'"></div>';}).join('');}
function execute(){const inp=document.getElementById('si').value.trim();if(!inp)return;execCount++;document.getElementById('sx').textContent=execCount;document.getElementById('sr').innerHTML='<div style="color:var(--text-dim)">Executing...</div>';ws.send(inp);addLog('exec','▶ '+inp.substring(0,60));}
function resetPlayground(){document.getElementById('si').value='';document.getElementById('sr').innerHTML='Ready';}
function showResult(text){document.getElementById('sr').innerHTML=text;}
let timelineData=[];
function addTimeline(status,dur){timelineData.push({status,dur:Math.max(1,dur||1)});if(timelineData.length>40)timelineData.shift();const el=document.getElementById('timeline'),maxD=timelineData.length?Math.max(...timelineData.map(d=>d.dur)):1;el.innerHTML=timelineData.map(d=>'<div class="bar" style="height:'+Math.max(3,(d.dur/maxD)*18)+'px;background:'+(d.status==='ok'?'var(--green)':'var(--red)')+'" title="'+d.dur+'ms"></div>').join('');}
let currentLayout='force';
function setLayout(type){document.querySelectorAll('.lbtn').forEach(b=>b.classList.remove('on'));document.getElementById('lb-'+type).classList.add('on');currentLayout=type;const cx=cv.width/2,cy=cv.height/2,r=Math.min(cx,cy)*.7,n=graphNodes.length;if(type==='circular'){graphNodes.forEach((nd,i)=>{const a=(i/n)*Math.PI*2-Math.PI/2;nd.x=cx+Math.cos(a)*r;nd.y=cy+Math.sin(a)*r;nd.vx=0;nd.vy=0;});}else if(type==='grid'){const cols=Math.ceil(Math.sqrt(n)),spacing=Math.min(cx,cy)*1.3/cols;graphNodes.forEach((nd,i)=>{nd.x=cx-spacing*(cols-1)/2+(i%cols)*spacing;nd.y=cy-spacing*(Math.ceil(n/cols)-1)/2+Math.floor(i/cols)*spacing;nd.vx=0;nd.vy=0;});}}
const cv=document.getElementById('gcv'),ctx=cv.getContext('2d'),tt=document.getElementById('tt'),gc=document.getElementById('gc');
function resizeC(){cv.width=gc.clientWidth;cv.height=gc.clientHeight;}window.addEventListener('resize',resizeC);resizeC();
function initGraph(){graphNodes.forEach(n=>{n.x=cv.width/2+(Math.random()-.5)*200;n.y=cv.height/2+(Math.random()-.5)*200;n.vx=0;n.vy=0;});}
if(graphNodes.length===0){graphNodes=[{id:'sys',label:'sys',val:.9,x:0,y:0,vx:0,vy:0},{id:'goal',label:'goal',val:.7,x:0,y:0,vx:0,vy:0}];graphEdges=[{from:'sys',to:'goal'}];}initGraph();
gc.addEventListener('wheel',e=>{e.preventDefault();const f=e.deltaY>0?.9:1.1;zoom*=f;panX-=(e.offsetX-panX)*(f-1);panY-=(e.offsetY-panY)*(f-1);});gc.addEventListener('mousedown',e=>{if(e.button===0){panning=true;panSX=e.clientX-panX;panSY=e.clientY-panY;}});window.addEventListener('mouseup',()=>panning=false);
window.addEventListener('mousemove',e=>{if(panning){panX=e.clientX-panSX;panY=e.clientY-panSY;return;}const mx=e.clientX-gc.getBoundingClientRect().left,my=e.clientY-gc.getBoundingClientRect().top;let found=null;graphNodes.forEach(n=>{const sx=(n.x+panX)*zoom,sy=(n.y+panY)*zoom;if(Math.sqrt((mx-sx)**2+(my-sy)**2)<12*zoom)found=n;});if(found){tt.style.display='block';tt.style.left=(e.clientX-gc.getBoundingClientRect().left+16)+'px';tt.style.top=(e.clientY-gc.getBoundingClientRect().top-8)+'px';tt.innerHTML='<div>'+esc(found.label||found.id)+'</div><div>val: '+((found.val||0)*100).toFixed(0)+'%</div>';}else{tt.style.display='none';}});gc.addEventListener('dblclick',()=>{zoom=1;panX=0;panY=0;});gc.addEventListener('mouseleave',()=>{tt.style.display='none';panning=false;});
function simStep(){if(currentLayout!=='force')return;const dt=.2,rep=4000,slen=100,sk=.04,damp=.88,cx=cv.width/2/zoom-panX/zoom,cy=cv.height/2/zoom-panY/zoom;for(let i=0;i<graphNodes.length;i++)for(let j=i+1;j<graphNodes.length;j++){let dx=graphNodes[i].x-graphNodes[j].x,dy=graphNodes[i].y-graphNodes[j].y,d=Math.sqrt(dx*dx+dy*dy)||1,f=rep/(d*d),fx=dx/d*f,fy=dy/d*f;graphNodes[i].vx+=fx*dt;graphNodes[i].vy+=fy*dt;graphNodes[j].vx-=fx*dt;graphNodes[j].vy-=fy*dt;}graphEdges.forEach(e=>{const s=graphNodes.find(n=>n.id===e.from),t=graphNodes.find(n=>n.id===e.to);if(!s||!t)return;let dx=t.x-s.x,dy=t.y-s.y,d=Math.sqrt(dx*dx+dy*dy)||1,f=(d-slen)*sk,fx=dx/d*f,fy=dy/d*f;s.vx+=fx*dt;s.vy+=fy*dt;t.vx-=fx*dt;t.vy-=fy*dt;});graphNodes.forEach(n=>{n.vx+=(cx-n.x)*.001*dt;n.vy+=(cy-n.y)*.001*dt;n.vx*=damp;n.vy*=damp;n.x+=n.vx*dt;n.y+=n.vy*dt;});}
function drawGraph(){ctx.clearRect(0,0,cv.width,cv.height);ctx.save();ctx.translate(panX*zoom,panY*zoom);ctx.scale(zoom,zoom);graphEdges.forEach(e=>{const s=graphNodes.find(n=>n.id===e.from),t=graphNodes.find(n=>n.id===e.to);if(!s||!t)return;ctx.beginPath();ctx.moveTo(s.x,s.y);ctx.lineTo(t.x,t.y);ctx.strokeStyle='rgba(0,229,255,.12)';ctx.lineWidth=1;ctx.stroke();});graphNodes.forEach(n=>{const g=ctx.createRadialGradient(n.x,n.y,6,n.x,n.y,20);g.addColorStop(0,'rgba(0,229,255,.25)');g.addColorStop(1,'rgba(0,229,255,0)');ctx.beginPath();ctx.arc(n.x,n.y,20,0,Math.PI*2);ctx.fillStyle=g;ctx.fill();ctx.beginPath();ctx.arc(n.x,n.y,6+(n.val||.5)*6,0,Math.PI*2);ctx.fillStyle='#10101a';ctx.fill();ctx.strokeStyle='#00e5ff';ctx.lineWidth=1.5;ctx.stroke();ctx.fillStyle='#c8c8d8';ctx.font='9px monospace';ctx.textAlign='center';ctx.fillText(n.label||n.id,n.x,n.y-14);});ctx.restore();}
function animate(){for(let i=0;i<5;i++)simStep();drawGraph();requestAnimationFrame(animate);}animate();
function toggleTheme(){document.documentElement.dataset.theme=document.documentElement.dataset.theme==='dark'?'light':'dark';}
function openShortcuts(){document.getElementById('shadow').classList.add('open');}
function closeShortcuts(){document.getElementById('shadow').classList.remove('open');}
function esc(s){const d=document.createElement('div');d.textContent=s;return d.innerHTML;}
document.addEventListener('keydown',e=>{if(e.key==='?'&&!e.ctrlKey&&document.activeElement===document.body){e.preventDefault();openShortcuts();}if(e.key==='Escape')closeShortcuts();if(e.key==='0'&&!e.ctrlKey){e.preventDefault();zoom=1;panX=0;panY=0;}if(e.key==='t'&&e.ctrlKey){e.preventDefault();toggleTheme();}if(e.key==='Enter'&&e.ctrlKey){e.preventDefault();execute();}});
</script></body></html>"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gateway::GatewayState;
    use std::sync::Mutex;

    #[test]
    fn test_dashboard_html_is_valid() {
        assert!(DASHBOARD_HTML.contains("<!DOCTYPE html>"));
        assert!(DASHBOARD_HTML.contains("A2X"));
        assert!(DASHBOARD_HTML.contains("chat-msgs"));
    }

    #[test]
    fn test_dashboard_has_model_selector() {
        assert!(DASHBOARD_HTML.contains("model-sel"));
    }

    #[test]
    fn test_build_snapshot_empty() {
        let state = Arc::new(HttpGatewayState {
            gateway: Arc::new(Mutex::new(GatewayState::new())),
        });
        let snap = build_snapshot(&state, 1).unwrap();
        assert_eq!(snap["type"], "snapshot");
        assert!(snap.get("vm_status").is_some());
    }

    #[test]
    fn test_execute_dashboard_program_empty() {
        let state = Arc::new(HttpGatewayState {
            gateway: Arc::new(Mutex::new(GatewayState::new())),
        });
        let result = execute_dashboard_program(&state, "");
        assert!(result.contains("empty result"));
    }

    #[test]
    fn test_bus_log_ring_buffer_capped() {
        let state = Arc::new(HttpGatewayState {
            gateway: Arc::new(Mutex::new(GatewayState::new())),
        });
        {
            let mut gw = state.gateway.lock().unwrap();
            for i in 0..250 {
                gw.record_bus_event("test", &format!("msg {}", i));
            }
        }
        let gw = state.gateway.lock().unwrap();
        assert_eq!(gw.clone_bus_log().len(), 200);
    }
}
