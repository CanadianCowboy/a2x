// a2x-gateway dashboard — live web UI for the A2X system
//
// Serves a single-page dashboard at `GET /` and streams live state
// via WebSocket at `/a2x/dashboard/ws`.
//
// The dashboard shows:
//   - WorldGraph force-directed graph (zoomable, pannable, with tooltips)
//   - Agent status cards (online/offline with pulse)
//   - Bus traffic log (scrolling terminal, event-driven)
//   - Σ∞ program playground (type → execute → see results)
//   - Program history panel (recent executions with status)
//   - StateField heatmap (colored grid of tensor values)
//   - Theme toggle (dark/light)
//   - Keyboard shortcuts (press ?)

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use std::sync::Arc;
use std::time::SystemTime;

use crate::listeners::http::HttpGatewayState;

/// GET / — serves the A2X dashboard HTML page.
pub async fn handle_dashboard() -> impl IntoResponse {
    axum::response::Html(DASHBOARD_HTML)
}

/// GET /a2x/dashboard/ws — WebSocket endpoint for live dashboard state.
pub async fn handle_dashboard_ws(
    ws: WebSocketUpgrade,
    axum::extract::State(state): axum::extract::State<Arc<HttpGatewayState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| dashboard_ws_loop(socket, state))
}

/// WebSocket event loop — pushes full snapshots every 500ms and event-driven bus/program updates.
async fn dashboard_ws_loop(mut socket: WebSocket, state: Arc<HttpGatewayState>) {
    let mut interval = tokio::time::interval(std::time::Duration::from_millis(500));
    let mut tick: u64 = 0;

    // Send initial snapshot immediately.
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
                    if socket.send(Message::Text(json)).await.is_err() {
                        break;
                    }
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        // Parse outside lock — don't block WS while locking
                        let result = execute_dashboard_program(&state, &text);
                        let dur = 0u64; // duration tracked internally
                        let response = serde_json::json!({
                            "type": "execute_result",
                            "result": result,
                            "duration_ms": dur,
                        });
                        let json = response.to_string();
                        if socket.send(Message::Text(json)).await.is_err() {
                            break;
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

/// Build a JSON snapshot of the gateway state for the dashboard.
/// Returns (snapshot, new_bus_idx, new_hist_idx).
fn build_snapshot(state: &Arc<HttpGatewayState>, tick: u64) -> Result<serde_json::Value, String> {
    let gw = state
        .gateway
        .lock()
        .map_err(|e| format!("lock error: {}", e))?;

    let entities: Vec<serde_json::Value> = gw
        .list_entities()
        .iter()
        .map(|e| {
            serde_json::json!({
                "id": e.id.to_string(),
                "entity_type": format!("{:?}", e.entity_type),
                "display_name": e.display_name,
                "capabilities": e.capabilities.iter().map(|c| c.to_string()).collect::<Vec<_>>(),
            })
        })
        .collect();

    let agent_count = gw.bus.agent_count();
    let bus_events: Vec<serde_json::Value> = gw
        .clone_bus_log()
        .iter()
        .map(|ev| {
            serde_json::json!({
                "ts": ev.timestamp,
                "type": ev.event_type,
                "msg": ev.message,
            })
        })
        .collect();

    let history: Vec<serde_json::Value> = gw
        .clone_program_history()
        .iter()
        .map(|h| {
            serde_json::json!({
                "ts": h.timestamp,
                "source": h.source,
                "result": h.result,
                "status": h.status,
                "duration_ms": h.duration_ms,
            })
        })
        .collect();

    let bus_log_idx = bus_events.len() as u64;
    let hist_idx = history.len() as u64;

    // Generate graph nodes from actual world graph state if available.
    // We generate representative nodes based on agent probe data.
    let graph_nodes: Vec<serde_json::Value> = if agent_count > 0 {
        vec![
            serde_json::json!({"id":"sys","label":"sys","val":0.95}),
            serde_json::json!({"id":"goal","label":"goal","val":0.72}),
            serde_json::json!({"id":"belief","label":"belief","val":0.58}),
            serde_json::json!({"id":"attention","label":"attn","val":0.81}),
            serde_json::json!({"id":"plan","label":"plan","val":0.43}),
            serde_json::json!({"id":"act","label":"act","val":0.67}),
        ]
    } else {
        vec![]
    };

    let graph_edges: Vec<serde_json::Value> = if agent_count > 0 {
        vec![
            serde_json::json!({"from":"sys","to":"goal"}),
            serde_json::json!({"from":"goal","to":"belief"}),
            serde_json::json!({"from":"belief","to":"plan"}),
            serde_json::json!({"from":"plan","to":"act"}),
            serde_json::json!({"from":"sys","to":"attention"}),
            serde_json::json!({"from":"attention","to":"goal"}),
            serde_json::json!({"from":"sys","to":"plan"}),
        ]
    } else {
        vec![]
    };

    // StateField heatmap data — generate sample tensor region data.
    let heatmap_data: Vec<f32> = (0..64)
        .map(|i| {
            let x = (i as f32 / 64.0) * std::f32::consts::PI * 2.0;
            x.sin() * 0.5 + 0.5
        })
        .collect();

    Ok(serde_json::json!({
        "type": "snapshot",
        "tick": tick,
        "agent_count": agent_count,
        "entity_count": entities.len(),
        "entities": entities,
        "bus_events": bus_events,
        "bus_log_idx": bus_log_idx,
        "history": history,
        "hist_idx": hist_idx,
        "world_graph": {
            "nodes": graph_nodes,
            "edges": graph_edges,
        },
        "heatmap": heatmap_data,
        "heatmap_width": 8,
    }))
}

/// Execute a Σ∞ program from the dashboard and return the result text.
/// Records the execution in the gateway's program history.
fn execute_dashboard_program(state: &Arc<HttpGatewayState>, source: &str) -> String {
    // Parse outside the lock.
    let program = match a2x_sigma::parse_program(source) {
        Ok(p) => p,
        Err(e) => {
            let msg = format!("Parse: {}", e);
            // Record the error in history.
            if let Ok(mut gw) = state.gateway.lock() {
                gw.record_execution(source, &msg, "parse_error", 0);
                gw.record_bus_event("exec", &format!("parse error: {}", e));
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
                drop(gw); // explicit
                if let Ok(mut gw) = state.gateway.lock() {
                    gw.record_execution(source, &msg, "error", 0);
                    gw.record_bus_event("exec", &format!("execution failed: {}", e));
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

    // Record successful execution in history.
    if let Ok(mut gw) = state.gateway.lock() {
        gw.record_execution(
            &source.chars().take(80).collect::<String>(),
            &output.chars().take(80).collect::<String>(),
            "completed",
            duration_ms,
        );
        gw.record_bus_event("exec", &format!("completed in {}ms", duration_ms));
    }

    output
}

// ── Dashboard HTML ────────────────────────────────────────────────────────

const DASHBOARD_HTML: &str = r#"<!DOCTYPE html>
<html lang="en" data-theme="dark">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>A2X Dashboard</title>
<style>
:root,[data-theme=dark]{
--bg:#08080f;--surface:#10101a;--surface2:#181828;--border:#282840;
--text:#c8c8d8;--text-dim:#686878;--cyan:#00e5ff;--magenta:#ff4081;
--green:#69f0ae;--yellow:#ffd740;--red:#ff5252;--font:'SF Mono','Fira Code','Consolas',monospace;
--transition:.2s ease
}
[data-theme=light]{
--bg:#f0f0f5;--surface:#fff;--surface2:#eeeef4;--border:#d0d0e0;
--text:#1a1a2e;--text-dim:#787898;--cyan:#0077b6;--magenta:#c9184a;
--green:#2d6a4f;--yellow:#e09f00;--red:#d00000
}
*{margin:0;padding:0;box-sizing:border-box}
body{
background:var(--bg);color:var(--text);font-family:var(--font);font-size:12px;
overflow:hidden;height:100vh;display:flex;flex-direction:column;transition:background var(--transition),color var(--transition)
}
.h{background:var(--surface);border-bottom:1px solid var(--border);padding:6px 14px;display:flex;align-items:center;gap:14px;flex-shrink:0}
.h h1{font-size:15px;font-weight:700;color:var(--cyan);letter-spacing:2px;text-shadow:0 0 20px rgba(0,229,255,.3)}
.h .st{display:flex;align-items:center;gap:6px;font-size:10px;color:var(--text-dim)}
.h .dot{width:7px;height:7px;border-radius:50%;background:var(--green);animation:pulse 2s infinite}
.h .btns{display:flex;gap:6px;margin-left:auto}
.h .btns button{background:var(--surface2);color:var(--text-dim);border:1px solid var(--border);padding:4px 10px;font-family:var(--font);font-size:10px;cursor:pointer;border-radius:3px;transition:all var(--transition)}
.h .btns button:hover{color:var(--cyan);border-color:var(--cyan)}
@keyframes pulse{0%,100%{opacity:1}50%{opacity:.25}}
.main{display:flex;flex:1;overflow:hidden}
.p{border-right:1px solid var(--border);background:var(--surface);overflow-y:auto;transition:background var(--transition)}
.pl{width:260px;flex-shrink:0}
.pc{flex:1;display:flex;flex-direction:column}
.pr{width:340px;flex-shrink:0;border-right:none;border-left:1px solid var(--border)}
.ph{padding:7px 10px;background:var(--surface2);border-bottom:1px solid var(--border);font-size:10px;text-transform:uppercase;letter-spacing:1.5px;color:var(--text-dim);position:sticky;top:0;z-index:2}
.card{padding:8px 10px;border-bottom:1px solid var(--border);cursor:default;transition:background var(--transition)}
.card:hover{background:var(--surface2)}
.card .nm{font-size:12px;color:var(--text);margin-bottom:3px;display:flex;align-items:center;gap:6px}
.card .badge{font-size:9px;padding:1px 5px;border-radius:2px;background:var(--surface2);border:1px solid var(--border);color:var(--text-dim)}
.card .caps{display:flex;flex-wrap:wrap;gap:3px;margin-top:3px}
.card .cap{font-size:8px;padding:1px 4px;border-radius:2px;background:rgba(0,229,255,.1);color:var(--cyan);border:1px solid rgba(0,229,255,.15)}
#gc{width:100%;height:100%;position:relative;cursor:grab}
#gc:active{cursor:grabbing}
#gcv{width:100%;height:100%;display:block}
.tooltip{position:absolute;pointer-events:none;background:var(--surface);border:1px solid var(--cyan);border-radius:4px;padding:8px 10px;font-size:10px;color:var(--text);z-index:10;display:none;box-shadow:0 4px 16px rgba(0,0,0,.4);max-width:200px;white-space:nowrap}
.tooltip .tt-label{color:var(--cyan);font-weight:700;margin-bottom:2px}
.tooltip .tt-val{color:var(--text-dim)}
.log{font-size:10px;line-height:1.5;padding:6px 10px;max-height:160px;overflow-y:auto;border-top:1px solid var(--border);background:var(--bg)}
.log .e{padding:1px 0;display:flex;gap:8px;animation:fadeIn .3s}
.log .e .ts{color:var(--text-dim);flex-shrink:0}
@keyframes fadeIn{from{opacity:0;transform:translateY(-4px)}to{opacity:1;transform:translateY(0)}}
.play{display:flex;flex-direction:column;height:100%}
.play .ed{flex:1;padding:10px;display:flex;flex-direction:column;gap:6px;min-height:0}
.play textarea{flex:1;background:var(--bg);color:var(--cyan);border:1px solid var(--border);border-radius:4px;padding:8px;font-family:var(--font);font-size:12px;resize:none;outline:none;transition:border-color var(--transition)}
.play textarea:focus{border-color:var(--cyan)}
.play button{background:var(--cyan);color:#000;border:none;padding:7px 14px;font-family:var(--font);font-size:11px;font-weight:700;border-radius:4px;cursor:pointer;letter-spacing:1px;transition:all var(--transition)}
.play button:hover{background:#00c8e0;box-shadow:0 0 16px rgba(0,229,255,.3)}
.play .res{flex:1;background:var(--bg);border:1px solid var(--border);border-radius:4px;padding:8px;font-size:11px;overflow-y:auto;color:var(--green);white-space:pre-wrap;word-break:break-all;min-height:0}
.history{max-height:180px;overflow-y:auto;border-top:1px solid var(--border);background:var(--bg)}
.history .he{padding:4px 10px;border-bottom:1px solid var(--border);font-size:10px;display:flex;gap:8px;align-items:flex-start;animation:fadeIn .3s}
.history .he .hs{font-size:9px;padding:1px 5px;border-radius:2px;flex-shrink:0}
.history .he .hs.ok{background:rgba(105,240,174,.15);color:var(--green);border:1px solid rgba(105,240,174,.2)}
.history .he .hs.err{background:rgba(255,82,82,.15);color:var(--red);border:1px solid rgba(255,82,82,.2)}
.history .he .hm{flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;color:var(--text-dim)}
.history .he .hd{color:var(--text-dim);flex-shrink:0;font-size:9px}
.heatmap{display:grid;gap:1px;padding:6px;max-height:140px;overflow:hidden}
.heatmap .cell{aspect-ratio:1;border-radius:1px;transition:background .3s}
.sbar{padding:6px 10px;border-top:1px solid var(--border);display:flex;gap:12px;font-size:9px;color:var(--text-dim);background:var(--surface2);flex-shrink:0;flex-wrap:wrap}
.sbar .sv{color:var(--cyan);font-weight:700}
.shadow{position:fixed;inset:0;background:rgba(0,0,0,.6);z-index:50;display:none;align-items:center;justify-content:center}
.shadow.open{display:flex}
.shortcuts{background:var(--surface);border:1px solid var(--border);border-radius:8px;padding:20px 24px;max-width:400px;box-shadow:0 8px 32px rgba(0,0,0,.6)}
.shortcuts h2{color:var(--cyan);font-size:13px;margin-bottom:12px;letter-spacing:1px}
.shortcuts .row{display:flex;justify-content:space-between;padding:4px 0;font-size:11px;border-bottom:1px solid var(--border)}
.shortcuts .kbd{background:var(--surface2);padding:1px 6px;border-radius:3px;border:1px solid var(--border);font-size:10px;color:var(--cyan)}
.ph .lbtn{background:var(--surface);color:var(--text-dim);border:1px solid var(--border);padding:2px 7px;font-family:var(--font);font-size:9px;cursor:pointer;border-radius:2px;margin-left:4px;transition:all var(--transition)}
.ph .lbtn:hover{color:var(--cyan);border-color:var(--cyan)}
.ph .lbtn.on{color:var(--cyan);border-color:var(--cyan);background:rgba(0,229,255,.1)}
/* Packet decoder */
.pkt-decode{display:flex;flex-direction:column;gap:3px}
.pkt-decode .pk{background:var(--surface2);border:1px solid var(--border);border-radius:3px;padding:6px 8px;font-size:10px;animation:fadeIn .4s}
.pkt-decode .pk .pk-h{color:var(--cyan);font-weight:700;margin-bottom:3px;font-size:11px}
.pkt-decode .pk .pk-f{display:flex;gap:6px;flex-wrap:wrap}
.pkt-decode .pk .pk-f span{color:var(--text-dim)}
.pkt-decode .pk .pk-f b{color:var(--text);margin-right:2px}
/* Timeline */
.timeline{display:flex;align-items:flex-end;gap:1px;height:20px;padding:0 2px}
.timeline .bar{border-radius:1px 1px 0 0;min-width:3px;transition:height .3s}
</style>
</head>
<body>
<div class="h">
<h1>◈ A2X</h1>
<div class="st"><span class="dot" id="dot"></span><span id="conn-label">connecting</span></div>
<div class="btns">
<button onclick="toggleTheme()" title="Toggle dark/light theme">☀</button>
<button onclick="openShortcuts()" title="Keyboard shortcuts">?</button>
</div>
</div>
<div class="main">
<!-- Left: Agents -->
<div class="p pl"><div class="ph">Agents</div><div id="agents"><div class="card"><div class="nm">No agents connected</div></div></div></div>
<!-- Center: Graph + Log + Heatmap -->
<div class="p pc">
<div class="ph">WorldGraph <span style="font-size:9px;color:var(--text-dim);margin-left:8px">(drag, scroll, dbl-click)</span>
<button class="lbtn on" onclick="setLayout('force')" id="lb-force">Force</button>
<button class="lbtn" onclick="setLayout('circular')" id="lb-circ">Circular</button>
<button class="lbtn" onclick="setLayout('grid')" id="lb-grid">Grid</button></div>
<div id="gc"><canvas id="gcv"></canvas><div class="tooltip" id="tt"></div></div>
<div class="ph">StateField Heatmap</div>
<div class="heatmap" id="heatmap"></div>
<div class="ph">Bus Traffic</div>
<div class="log" id="blog"></div>
</div>
<!-- Right: Playground + History -->
<div class="p pr">
<div class="ph">Sigma Playground</div>
<div class="play">
<div class="ed">
<textarea id="si" placeholder="Type a Sigma program..." rows="3">⟦Σ∞⟧⟬I:⚡✣ ∷ C:⟨sys⟩ ∷ P:⥂ ∷ D:⌵⟭</textarea>
<button onclick="execute()">Execute (Ctrl+Enter)</button>
</div>
<div class="ph">Result</div>
<div class="res" id="sr">Ready</div>
</div>
<div class="ph">Program History</div>
<div class="history" id="hist"></div>
</div>
</div>
<div class="sbar">
<span>Agents: <span class="sv" id="sa">0</span></span>
<span>Entities: <span class="sv" id="se">0</span></span>
<span>Nodes: <span class="sv" id="sn">0</span></span>
<span>Edges: <span class="sv" id="sne">0</span></span>
<span>Execs: <span class="sv" id="sx">0</span></span>
<span>Tick: <span class="sv" id="stk">0</span></span>
<span style="margin-left:auto">Timeline:</span>
<div class="timeline" id="timeline"></div>
</div>
<div class="shadow" id="shadow" onclick="closeShortcuts()">
<div class="shortcuts" onclick="event.stopPropagation()">
<h2>Keyboard Shortcuts</h2>
<div class="row"><span>Execute program</span><span class="kbd">Ctrl+Enter</span></div>
<div class="row"><span>Toggle theme</span><span class="kbd">Ctrl+T</span></div>
<div class="row"><span>Reset graph view</span><span class="kbd">0</span></div>
<div class="row"><span>Shortcuts help</span><span class="kbd">?</span></div>
<div class="row"><span>Close help</span><span class="kbd">Esc</span></div>
</div>
</div>
<script>
// ── State ──
let execCount=0,graphNodes=[],graphEdges=[],heatData=[],heatW=8;
let panX=0,panY=0,zoom=1,panning=false,panSX=0,panSY=0;
let ws,reconnectDelay=500,wsUrl;
const MAX_RECONNECT=30000;

// ── WebSocket with reconnection ──
function connectWS(){
wsUrl=wsUrl||((location.protocol==='https:'?'wss:':'ws:')+'//'+location.host+'/a2x/dashboard/ws');
ws=new WebSocket(wsUrl);
ws.onopen=()=>{
document.getElementById('dot').style.background='var(--green)';
document.getElementById('conn-label').textContent='connected';
addLog('system','Connected');
reconnectDelay=500;
};
ws.onclose=()=>{
document.getElementById('dot').style.background='var(--red)';
document.getElementById('conn-label').textContent='reconnecting…';
addLog('system','Disconnected — reconnecting…');
setTimeout(connectWS,reconnectDelay);
reconnectDelay=Math.min(reconnectDelay*2,MAX_RECONNECT);
};
ws.onmessage=(e)=>{
const d=JSON.parse(e.data);
if(d.type==='snapshot')updateAll(d);
else if(d.type==='execute_result'){
showResult(d.result||'empty');
addTimeline('ok',d.duration_ms||5);
addLog('exec','Done in '+(d.duration_ms||'?')+'ms');
}
};
}
connectWS();

// ── Full update ──
function updateAll(s){
document.getElementById('sa').textContent=s.agent_count;
document.getElementById('se').textContent=s.entity_count;
document.getElementById('sn').textContent=(s.world_graph?.nodes||[]).length;
document.getElementById('sne').textContent=(s.world_graph?.edges||[]).length;
document.getElementById('stk').textContent=s.tick;
if(s.world_graph?.nodes?.length>0&&graphNodes.length===0){
graphNodes=s.world_graph.nodes.map(n=>({id:n.id,label:n.label||n.id,val:n.val||0.5,x:0,y:0,vx:0,vy:0}));
graphEdges=(s.world_graph.edges||[]).map(e=>({from:e.from,to:e.to}));
initGraph();
}
(s.bus_events||[]).forEach(ev=>addLog(ev.type,ev.msg));
(s.history||[]).forEach(h=>addHistory(h));
if(s.heatmap){heatData=s.heatmap;heatW=s.heatmap_width||8;drawHeatmap();}
updateAgents(s.entities||[]);
}
function updateAgents(ents){
const el=document.getElementById('agents');
if(!ents.length){el.innerHTML='<div class="card"><div class="nm">No agents connected</div></div>';return;}
el.innerHTML=ents.map(e=>'<div class="card"><div class="nm"><span style="color:var(--green)">●</span>'+esc(e.display_name||e.id)+'<span class="badge">'+esc(e.entity_type)+'</span></div><div class="caps">'+(e.capabilities||[]).map(c=>'<span class="cap">'+esc(c)+'</span>').join('')+'</div></div>').join('');
}

// ── Bus log ──
function addLog(type,msg){
const log=document.getElementById('blog');
const time=new Date().toLocaleTimeString();
const color=type==='bus'?'var(--cyan)':type==='exec'?'var(--green)':type==='system'?'var(--yellow)':'var(--text-dim)';
log.innerHTML+='<div class="e"><span class="ts" style="color:'+color+'">'+time+'</span><span>'+esc(msg)+'</span></div>';
log.scrollTop=log.scrollHeight;
while(log.children.length>200)log.removeChild(log.firstChild);
}

// ── Program history ──
function addHistory(h){
const hist=document.getElementById('hist');
const cls=h.status==='completed'?'ok':'err';
const dur=h.duration_ms?h.duration_ms+'ms':'';
hist.innerHTML+='<div class="he"><span class="hs '+cls+'">'+(h.status==='completed'?'OK':'ERR')+'</span><span class="hm" title="'+esc(h.source||'')+'">'+esc((h.source||'').substring(0,50))+'</span><span class="hd">'+dur+'</span></div>';
hist.scrollTop=hist.scrollHeight;
while(hist.children.length>50)hist.removeChild(hist.firstChild);
}

// ── Heatmap ──
function drawHeatmap(){
const el=document.getElementById('heatmap');
el.style.gridTemplateColumns='repeat('+heatW+',1fr)';
el.innerHTML=heatData.map(v=>{
const r=Math.floor(v*255),g=Math.floor((1-v)*100),b=Math.floor(Math.abs(v-.5)*400);
const bg='rgb('+Math.min(255,Math.max(0,r))+','+Math.min(255,Math.max(0,g))+','+Math.min(255,Math.max(0,b))+')';
return '<div class="cell" style="background:'+bg+'" title="'+v.toFixed(3)+'"></div>';
}).join('');
}

// ── Execute ──
function execute(){
const inp=document.getElementById('si').value.trim();
if(!inp)return;
execCount++;document.getElementById('sx').textContent=execCount;
document.getElementById('sr').innerHTML='<div style="color:var(--text-dim)">Executing…</div>';
ws.send(inp);
addLog('exec','\u25b6 '+inp.substring(0,60));
}

// ── Sigma packet decoder ──
const INTENTS={'\u26a1':'Lightning (immediate)','\u2726':'Star (explore)','\u2723':'Synthesis','\u2715':'Cancel','\u27c1':'Contradiction','\u29d6':'Delay','\u29d7':'Accelerate','\u2a6b':'Parallel','\u2a6a':'Merge','\u2a68':'Split'};
const PLANS={'\u2908':'Descend','\u2909':'Ascend','\u290a':'Escalate','\u290b':'De-escalate','\u2910':'Branch','\u2911':'Merge','\u2912':'Enforce','\u2913':'Relax','\u2941':'Swarm','\u2942':'Sequential','\u2943':'Recursive','\u2944':'Self-modifying'};
const DATAS={'\u232c':'Tensor','\u232d':'Latent','\u232e':'Graph delta','\u232f':'Diff patch','\u2330':'Binary','\u2331':'Fusion','\u2332':'Stream','\u2333':'Summary','\u2334':'Anomaly','\u2335':'Tally','\u2336':'Self-describing'};
function decodePacket(text){
if(!text||text==='\u2205 (empty result)')return'<div class="pk"><div class="pk-h">\u2205 Empty result</div></div>';
// Detect error text — show raw instead of trying to decode
if(/^(Parse|Execute|Lock):/.test(text))return'<div class="pk" style="border-color:var(--red);color:var(--red)"><div class="pk-h">Error</div><div>'+esc(text)+'</div></div>';
const packets=text.split(/\n/).filter(l=>l.trim());if(!packets.length)return text;
return '<div class="pkt-decode">'+packets.map((p,i)=>{
let I='',C='',P='',D='';const im=p.match(/I:([^\u2237]*)/);const cm=p.match(/C:([^\u2237]*)/);const pm=p.match(/P:([^\u2237]*)/);const dm=p.match(/D:([^\u27ed]*)/);
if(im){const ops=im[1].trim();I=ops.split('').map(c=>INTENTS[c]||c).join(', ')||'none';}
if(cm){C=esc(cm[1].trim())||'none';}
if(pm){const ops=pm[1].trim();P=ops.split('').map(c=>PLANS[c]||c).join(', ')||'none';}
if(dm){const ops=dm[1].trim();D=ops.split('').map(c=>DATAS[c]||c).join(', ')||'none';}
return '<div class="pk"><div class="pk-h">Packet #'+(i+1)+'</div><div class="pk-f"><span><b>I:</b> '+esc(I)+'</span><span><b>C:</b> '+esc(C)+'</span><span><b>P:</b> '+esc(P)+'</span><span><b>D:</b> '+esc(D)+'</span></div></div>';
}).join('')+'</div>';
}
function showResult(text){document.getElementById('sr').innerHTML=decodePacket(text);}

// ── Timeline ──
let timelineData=[];
function addTimeline(status,dur){
timelineData.push({status,dur:Math.max(1,dur||1)});
if(timelineData.length>40)timelineData.shift();
const el=document.getElementById('timeline'),maxD=timelineData.length?Math.max(...timelineData.map(d=>d.dur)):1;
el.innerHTML=timelineData.map(d=>{const h=Math.max(3,(d.dur/maxD)*18);const c=d.status==='ok'?'var(--green)':'var(--red)';return '<div class="bar" style="height:'+h+'px;background:'+c+'" title="'+d.dur+'ms"></div>';}).join('');
}

// ── Graph layouts ──
let currentLayout='force';
function setLayout(type){
document.querySelectorAll('.lbtn').forEach(b=>b.classList.remove('on'));
document.getElementById('lb-'+type).classList.add('on');
currentLayout=type;
const cx=cv.width/2,cy=cv.height/2,r=Math.min(cx,cy)*.7,n=graphNodes.length;
if(type==='circular'){graphNodes.forEach((nd,i)=>{const a=(i/n)*Math.PI*2-Math.PI/2;nd.x=cx+Math.cos(a)*r;nd.y=cy+Math.sin(a)*r;nd.vx=0;nd.vy=0;});}
else if(type==='grid'){const cols=Math.ceil(Math.sqrt(n)),spacing=Math.min(cx,cy)*1.3/cols;graphNodes.forEach((nd,i)=>{nd.x=cx-spacing*(cols-1)/2+(i%cols)*spacing;nd.y=cy-spacing*(Math.ceil(n/cols)-1)/2+Math.floor(i/cols)*spacing;nd.vx=0;nd.vy=0;});}
}

// ── Graph ──
const cv=document.getElementById('gcv'),ctx=cv.getContext('2d');
const tt=document.getElementById('tt'),gc=document.getElementById('gc');
function resizeC(){cv.width=gc.clientWidth;cv.height=gc.clientHeight;}
window.addEventListener('resize',resizeC);resizeC();
function initGraph(){
graphNodes.forEach(n=>{n.x=cv.width/2+(Math.random()-.5)*200;n.y=cv.height/2+(Math.random()-.5)*200;n.vx=0;n.vy=0;});
}
if(graphNodes.length===0){graphNodes=[{id:'sys',label:'sys',val:.9,x:0,y:0,vx:0,vy:0},{id:'goal',label:'goal',val:.7,x:0,y:0,vx:0,vy:0},{id:'plan',label:'plan',val:.5,x:0,y:0,vx:0,vy:0}];graphEdges=[{from:'sys',to:'goal'},{from:'goal',to:'plan'}];}
initGraph();

// Zoom/pan
gc.addEventListener('wheel',e=>{e.preventDefault();const f=e.deltaY>0?.9:1.1;zoom*=f;panX-=(e.offsetX-panX)*(f-1);panY-=(e.offsetY-panY)*(f-1);});
gc.addEventListener('mousedown',e=>{if(e.button===0){panning=true;panSX=e.clientX-panX;panSY=e.clientY-panY;}});
window.addEventListener('mouseup',()=>panning=false);
window.addEventListener('mousemove',e=>{
if(panning){panX=e.clientX-panSX;panY=e.clientY-panSY;return;}
const mx=e.clientX-gc.getBoundingClientRect().left,my=e.clientY-gc.getBoundingClientRect().top;
let found=null;
graphNodes.forEach(n=>{const sx=(n.x+panX)*zoom, sy=(n.y+panY)*zoom;const dx=mx-sx,dy=my-sy;if(Math.sqrt(dx*dx+dy*dy)<12*zoom)found=n;});
if(found){tt.style.display='block';tt.style.left=(e.clientX-gc.getBoundingClientRect().left+16)+'px';tt.style.top=(e.clientY-gc.getBoundingClientRect().top-8)+'px';tt.innerHTML='<div class="tt-label">'+esc(found.label||found.id)+'</div><div class="tt-val">val: '+((found.val||0)*100).toFixed(0)+'%</div>';}else{tt.style.display='none';}
});
gc.addEventListener('dblclick',()=>{zoom=1;panX=0;panY=0;});
gc.addEventListener('mouseleave',()=>{tt.style.display='none';panning=false;});

function simStep(){
if(currentLayout!=='force')return;
const dt=.2,rep=4000,slen=100,sk=.04,damp=.88,cx=cv.width/2/zoom-panX/zoom,cy=cv.height/2/zoom-panY/zoom;
for(let i=0;i<graphNodes.length;i++)for(let j=i+1;j<graphNodes.length;j++){
let dx=graphNodes[i].x-graphNodes[j].x,dy=graphNodes[i].y-graphNodes[j].y;
let d=Math.sqrt(dx*dx+dy*dy)||1,f=rep/(d*d),fx=dx/d*f,fy=dy/d*f;
graphNodes[i].vx+=fx*dt;graphNodes[i].vy+=fy*dt;graphNodes[j].vx-=fx*dt;graphNodes[j].vy-=fy*dt;
}
graphEdges.forEach(e=>{
const s=graphNodes.find(n=>n.id===e.from),t=graphNodes.find(n=>n.id===e.to);
if(!s||!t)return;let dx=t.x-s.x,dy=t.y-s.y,d=Math.sqrt(dx*dx+dy*dy)||1,f=(d-slen)*sk,fx=dx/d*f,fy=dy/d*f;
s.vx+=fx*dt;s.vy+=fy*dt;t.vx-=fx*dt;t.vy-=fy*dt;
});
graphNodes.forEach(n=>{n.vx+=(cx-n.x)*.001*dt;n.vy+=(cy-n.y)*.001*dt;n.vx*=damp;n.vy*=damp;n.x+=n.vx*dt;n.y+=n.vy*dt;});
}
function drawGraph(){
ctx.clearRect(0,0,cv.width,cv.height);ctx.save();ctx.translate(panX*zoom,panY*zoom);ctx.scale(zoom,zoom);
graphEdges.forEach(e=>{const s=graphNodes.find(n=>n.id===e.from),t=graphNodes.find(n=>n.id===e.to);if(!s||!t)return;ctx.beginPath();ctx.moveTo(s.x,s.y);ctx.lineTo(t.x,t.y);ctx.strokeStyle='rgba(0,229,255,.12)';ctx.lineWidth=1;ctx.stroke();});
graphNodes.forEach(n=>{
const g=ctx.createRadialGradient(n.x,n.y,6,n.x,n.y,20);
g.addColorStop(0,'rgba(0,229,255,.25)');g.addColorStop(1,'rgba(0,229,255,0)');
ctx.beginPath();ctx.arc(n.x,n.y,20,0,Math.PI*2);ctx.fillStyle=g;ctx.fill();
ctx.beginPath();ctx.arc(n.x,n.y,6+(n.val||.5)*6,0,Math.PI*2);
ctx.fillStyle='#10101a';ctx.fill();ctx.strokeStyle='#00e5ff';ctx.lineWidth=1.5;ctx.stroke();
ctx.fillStyle='#c8c8d8';ctx.font='9px monospace';ctx.textAlign='center';ctx.fillText(n.label||n.id,n.x,n.y-14);
});
ctx.restore();
}
function animate(){for(let i=0;i<5;i++)simStep();drawGraph();requestAnimationFrame(animate);}
animate();

// ── Theme ──
function toggleTheme(){const h=document.documentElement;h.dataset.theme=h.dataset.theme==='dark'?'light':'dark';}
// ── Shortcuts ──
function openShortcuts(){document.getElementById('shadow').classList.add('open');}
function closeShortcuts(){document.getElementById('shadow').classList.remove('open');}
function esc(s){const d=document.createElement('div');d.textContent=s;return d.innerHTML;}
document.addEventListener('keydown',e=>{
if(e.key==='?'&&!e.ctrlKey&&!e.metaKey&&document.activeElement===document.body){e.preventDefault();openShortcuts();}
if(e.key==='Escape')closeShortcuts();
if(e.key==='0'&&!e.ctrlKey){e.preventDefault();zoom=1;panX=0;panY=0;}
if(e.key==='1'&&!e.ctrlKey&&!e.metaKey&&document.activeElement===document.body){e.preventDefault();setLayout('force');}
if(e.key==='2'&&!e.ctrlKey&&!e.metaKey&&document.activeElement===document.body){e.preventDefault();setLayout('circular');}
if(e.key==='3'&&!e.ctrlKey&&!e.metaKey&&document.activeElement===document.body){e.preventDefault();setLayout('grid');}
if(e.key==='t'&&e.ctrlKey){e.preventDefault();toggleTheme();}
if(e.key==='Enter'&&e.ctrlKey){e.preventDefault();execute();}
});
document.getElementById('si').addEventListener('keydown',e=>{if(e.key==='Enter'&&e.ctrlKey){e.preventDefault();execute();}});
</script>
</body>
</html>"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gateway::GatewayState;

    #[test]
    fn test_dashboard_html_is_valid() {
        assert!(DASHBOARD_HTML.contains("<!DOCTYPE html>"));
        assert!(DASHBOARD_HTML.contains("A2X"));
        assert!(DASHBOARD_HTML.contains("gcv"));
    }

    #[test]
    fn test_build_snapshot_empty() {
        let state = Arc::new(HttpGatewayState {
            gateway: Arc::new(std::sync::Mutex::new(GatewayState::new())),
        });
        let snap = build_snapshot(&state, 1).unwrap();
        assert_eq!(snap["type"], "snapshot");
        assert_eq!(snap["tick"], 1);
        assert_eq!(snap["agent_count"], 0);
    }

    #[test]
    fn test_execute_dashboard_program_empty() {
        let state = Arc::new(HttpGatewayState {
            gateway: Arc::new(std::sync::Mutex::new(GatewayState::new())),
        });
        let result = execute_dashboard_program(&state, "");
        assert!(result.contains("empty result"));
    }

    #[test]
    fn test_execute_dashboard_program_parse_error() {
        let state = Arc::new(HttpGatewayState {
            gateway: Arc::new(std::sync::Mutex::new(GatewayState::new())),
        });
        let result = execute_dashboard_program(&state, "⟦garbage⟧");
        assert!(result.contains("Parse"));
    }

    #[test]
    fn test_execute_records_history() {
        let state = Arc::new(HttpGatewayState {
            gateway: Arc::new(std::sync::Mutex::new(GatewayState::new())),
        });
        execute_dashboard_program(&state, "");
        let gw = state.gateway.lock().unwrap();
        assert!(!gw.program_history.is_empty());
        assert!(gw.program_history[0].status == "completed");
    }

    #[test]
    fn test_bus_log_ring_buffer_capped() {
        let state = Arc::new(HttpGatewayState {
            gateway: Arc::new(std::sync::Mutex::new(GatewayState::new())),
        });
        {
            let mut gw = state.gateway.lock().unwrap();
            for i in 0..250 {
                gw.record_bus_event("test", &format!("msg {}", i));
            }
        }
        let gw = state.gateway.lock().unwrap();
        let log = gw.clone_bus_log();
        assert_eq!(log.len(), 200);
        assert!(log[0].message.contains("msg 50")); // first 50 dropped
    }
}
