// MCP (Model Context Protocol) Bridge — exposes A2X tools to LLMs via JSON-RPC 2.0.
// See plans/02-omega-compiler.md §6 and the comprehensive audit T6-2.
//
// Implements an MCP server over stdio that any MCP-compatible client (Claude,
// Continue, Cursor, etc.) can connect to. Exposes A2X's compile, validate, and
// decompile pipelines as discoverable tools.
//
// Protocol: JSON-RPC 2.0, newline-delimited JSON over stdin/stdout.
// MCP spec: modelcontextprotocol.io/specification/2025-11-25

use serde::Deserialize;
use std::io::{BufRead, BufReader, Write};

use crate::bridge::Bridge;
use crate::compiler::CompileToOmega;
use crate::passes::OptimizationLevel;
use crate::semantic;
use a2x_sigma::parse_program;

/// JSON-RPC 2.0 request envelope (only needs Deserialize).
#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    #[serde(default)]
    id: Option<serde_json::Value>,
    method: String,
    #[serde(default)]
    params: Option<serde_json::Value>,
}

// ── JSON-RPC 2.0 response helpers ──────────────────────────────────────────

/// Build a successful JSON-RPC response.
fn ok_response(id: Option<&serde_json::Value>, result: serde_json::Value) -> String {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result,
    })
    .to_string()
}

/// Build a JSON-RPC error response.
fn err_response(id: Option<&serde_json::Value>, code: i32, message: &str) -> String {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message,
        },
    })
    .to_string()
}

// ── MCP lifecycle handlers ─────────────────────────────────────────────────

/// Handle the `initialize` method — MCP handshake.
fn handle_initialize(id: Option<&serde_json::Value>) -> String {
    let result = serde_json::json!({
        "protocolVersion": "2025-03-26",
        "capabilities": {
            "tools": { "listChanged": false }
        },
        "serverInfo": {
            "name": "a2x-mcp-bridge",
            "version": env!("CARGO_PKG_VERSION")
        }
    });
    ok_response(id, result)
}

/// Handle the `tools/list` method.
fn handle_tools_list(id: Option<&serde_json::Value>) -> String {
    let tools = vec![
        serde_json::json!({
            "name": "compile_program",
            "description": "Compile a Σ∞ (Sigma Infinity) program into Ω latent tensors. Takes raw Σ∞ source text and returns the compiled Omega program with instruction count and source ID.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": {
                        "type": "string",
                        "description": "Raw Σ∞ source text. Example: ⟦Σ∞⟧⟬I:⚡ ∷ C:⟨sys⟩ ∷ P:⥂ ∷ D:⌬⟭"
                    },
                    "optimization_level": {
                        "type": "string",
                        "enum": ["none", "light", "default", "aggressive"],
                        "description": "Optimization level: none/light/default/aggressive"
                    }
                },
                "required": ["source"]
            }
        }),
        serde_json::json!({
            "name": "validate_program",
            "description": "Validate Σ∞ source for semantic correctness without compiling. Checks empty intents, contradictory operators, undefined jump targets, and type mismatches.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": {
                        "type": "string",
                        "description": "Raw Σ∞ source text to validate."
                    }
                },
                "required": ["source"]
            }
        }),
        serde_json::json!({
            "name": "decompile_packet",
            "description": "Decompile an Ω latent packet back to its Σ∞ symbolic form. Note: only the intent (I) region is reliably recoverable.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "packet_index": {
                        "type": "integer",
                        "description": "Which instruction in the compiled program to decompile (0-indexed)."
                    },
                    "source": {
                        "type": "string",
                        "description": "Σ∞ source text to compile first, then decompile the specified packet."
                    }
                },
                "required": ["source", "packet_index"]
            }
        }),
        serde_json::json!({
            "name": "get_info",
            "description": "Get information about the A2X system: version, architecture, supported protocols, and crate list.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
    ];

    ok_response(id, serde_json::json!({ "tools": tools }))
}

// ── Tool dispatch ──────────────────────────────────────────────────────────

/// Handle the `tools/call` method — execute a tool.
fn handle_tools_call(id: Option<&serde_json::Value>, params: &serde_json::Value) -> String {
    let tool_name = match params.get("name").and_then(|n| n.as_str()) {
        Some(n) => n,
        None => return err_response(id, -32602, "missing tool name"),
    };

    let arguments = params.get("arguments").unwrap_or(&serde_json::Value::Null);

    match tool_name {
        "compile_program" => exec_compile_program(id, arguments),
        "validate_program" => exec_validate_program(id, arguments),
        "decompile_packet" => exec_decompile_packet(id, arguments),
        "get_info" => exec_get_info(id),
        _ => err_response(id, -32601, &format!("unknown tool: {}", tool_name)),
    }
}

// ── Tool implementations ───────────────────────────────────────────────────

fn exec_compile_program(id: Option<&serde_json::Value>, args: &serde_json::Value) -> String {
    let source = match args.get("source").and_then(|s| s.as_str()) {
        Some(s) => s,
        None => return tool_error(id, "missing required parameter: source"),
    };

    let level = match args.get("optimization_level").and_then(|o| o.as_str()) {
        Some("none") | None => OptimizationLevel::None,
        Some("light") => OptimizationLevel::Light,
        Some("default") => OptimizationLevel::default(),
        Some("aggressive") => OptimizationLevel::Aggressive,
        Some(other) => return tool_error(id, &format!("unknown optimization level: {}", other)),
    };

    match parse_program(source) {
        Ok(prog) => match prog.compile(level) {
            Ok(omega) => {
                let result = serde_json::json!({
                    "instruction_count": omega.len(),
                    "source_id": omega.source_id.map(|id| id.to_string()),
                    "protocol": "Ω",
                    "compilation_success": true
                });
                tool_success(id, &result)
            }
            Err(e) => tool_error(id, &format!("compilation error: {}", e)),
        },
        Err(e) => tool_error(id, &format!("parse error: {}", e)),
    }
}

fn exec_validate_program(id: Option<&serde_json::Value>, args: &serde_json::Value) -> String {
    let source = match args.get("source").and_then(|s| s.as_str()) {
        Some(s) => s,
        None => return tool_error(id, "missing required parameter: source"),
    };

    match parse_program(source) {
        Ok(prog) => match semantic::analyze(&prog) {
            Ok(()) => {
                let result = serde_json::json!({
                    "valid": true,
                    "instruction_count": prog.instructions.len(),
                    "message": "program is semantically valid"
                });
                tool_success(id, &result)
            }
            Err(e) => {
                let result = serde_json::json!({
                    "valid": false,
                    "error": e.to_string()
                });
                tool_success_with_error(id, &result)
            }
        },
        Err(e) => tool_error(id, &format!("parse error: {}", e)),
    }
}

fn exec_decompile_packet(id: Option<&serde_json::Value>, args: &serde_json::Value) -> String {
    let source = match args.get("source").and_then(|s| s.as_str()) {
        Some(s) => s,
        None => return tool_error(id, "missing required parameter: source"),
    };

    let packet_index = match args.get("packet_index").and_then(|i| i.as_u64()) {
        Some(i) => i as usize,
        None => return tool_error(id, "missing required parameter: packet_index"),
    };

    match parse_program(source) {
        Ok(prog) => match prog.compile(OptimizationLevel::default()) {
            Ok(omega) => {
                if packet_index >= omega.instructions.len() {
                    return tool_error(
                        id,
                        &format!(
                            "packet_index {} out of range (program has {} instructions)",
                            packet_index,
                            omega.instructions.len()
                        ),
                    );
                }
                match Bridge::decompile(&omega.instructions[packet_index]) {
                    Some(packet) => {
                        let result = serde_json::json!({
                            "decompiled": packet.to_string(),
                            "intent_operators": packet.intent.operators.iter().map(|op| format!("{:?}", op)).collect::<Vec<_>>(),
                            "plan_operators": packet.plan.operators.iter().map(|op| format!("{:?}", op)).collect::<Vec<_>>(),
                            "context_labels": packet.context.labels,
                            "packet_index": packet_index
                        });
                        tool_success(id, &result)
                    }
                    None => tool_error(
                        id,
                        "decompile failed: could not recover Σ∞ from this Ω packet",
                    ),
                }
            }
            Err(e) => tool_error(id, &format!("compilation error: {}", e)),
        },
        Err(e) => tool_error(id, &format!("parse error: {}", e)),
    }
}

fn exec_get_info(id: Option<&serde_json::Value>) -> String {
    let result = serde_json::json!({
        "project": "A2X — Agent-to-Anything",
        "version": env!("CARGO_PKG_VERSION"),
        "description": "An AI-native programming language + runtime. Σ∞ (hyper-symbolic ISA) → Ω (compiled latent representation) → CCS (cognitive runtime VM).",
        "architecture": {
            "sigma": "Hyper-symbolic ISA — packets are instructions, sequences are programs",
            "omega": "Compiled latent tensors — pure neural representation, no symbols",
            "ccs": "CryoCore Cognitive Substrate — WorldGraph (heap), StateField (registers), MemoryTrace (execution log)"
        },
        "protocols": ["Σ∞ (sigma)", "Ω (omega)"],
        "supported_tools": ["compile_program", "validate_program", "decompile_packet", "get_info"],
        "crates": [
            "a2x-core", "a2x-sigma", "a2x-omega", "a2x-bus",
            "a2x-ccs", "a2x-agents", "a2x-gateway", "a2x-cli",
            "a2x-probe", "a2x-client"
        ]
    });
    tool_success(id, &result)
}

// ── MCP result formatting ──────────────────────────────────────────────────

/// Build a tool success result.
fn tool_success(id: Option<&serde_json::Value>, result: &serde_json::Value) -> String {
    let text = serde_json::to_string_pretty(result).unwrap_or_else(|_| format!("{:?}", result));
    let mcp_result = serde_json::json!({
        "content": [{ "type": "text", "text": text }],
        "isError": false
    });
    ok_response(id, mcp_result)
}

/// Build a tool result carrying error information (e.g. validation failure).
fn tool_success_with_error(id: Option<&serde_json::Value>, result: &serde_json::Value) -> String {
    let text = serde_json::to_string_pretty(result).unwrap_or_else(|_| format!("{:?}", result));
    let mcp_result = serde_json::json!({
        "content": [{ "type": "text", "text": text }],
        "isError": true
    });
    ok_response(id, mcp_result)
}

/// Build a tool error result (missing params, parse failure, etc.).
fn tool_error(id: Option<&serde_json::Value>, message: &str) -> String {
    let mcp_result = serde_json::json!({
        "content": [{ "type": "text", "text": message }],
        "isError": true
    });
    ok_response(id, mcp_result)
}

// ── Public API ─────────────────────────────────────────────────────────────

/// The MCP bridge — a JSON-RPC 2.0 server over stdio.
///
/// Reads newline-delimited JSON-RPC requests from stdin, processes them
/// against the A2X toolset, and writes responses to stdout.
///
/// ## Usage
///
/// ```ignore
/// use a2x_omega::mcp_bridge::McpBridge;
/// McpBridge::run(); // blocking, reads stdin until EOF
/// ```
pub struct McpBridge;

impl McpBridge {
    /// Run the MCP bridge server — reads from stdin, writes to stdout.
    /// Blocking. Processes one JSON-RPC message per line until stdin closes.
    pub fn run() {
        let stdin = std::io::stdin();
        let reader = BufReader::new(stdin.lock());
        let mut stdout = std::io::stdout().lock();

        for line_result in reader.lines() {
            let line = match line_result {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("mcp-bridge: read error: {}", e);
                    break;
                }
            };

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let response = Self::process_message(trimmed);
            if response.is_empty() {
                continue;
            }

            if writeln!(stdout, "{}", response).is_err() {
                break;
            }
            if stdout.flush().is_err() {
                break;
            }
        }
    }

    /// Process a single JSON-RPC 2.0 message.
    fn process_message(line: &str) -> String {
        let request: JsonRpcRequest = match serde_json::from_str(line) {
            Ok(r) => r,
            Err(e) => return err_response(None, -32700, &format!("parse error: {}", e)),
        };

        // Notifications (no id) produce no response.
        if request.id.is_none() {
            return String::new();
        }

        let id = &request.id;

        match request.method.as_str() {
            "initialize" => handle_initialize(id.as_ref()),
            "tools/list" => handle_tools_list(id.as_ref()),
            "tools/call" => {
                let params = match &request.params {
                    Some(p) => p,
                    None => return err_response(id.as_ref(), -32602, "missing params"),
                };
                handle_tools_call(id.as_ref(), params)
            }
            "ping" => ok_response(id.as_ref(), serde_json::json!({})),
            _ => err_response(
                id.as_ref(),
                -32601,
                &format!("method not found: {}", request.method),
            ),
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_initialize() {
        let id = serde_json::Value::Number(1.into());
        let response = handle_initialize(Some(&id));
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed["jsonrpc"], "2.0");
        assert_eq!(parsed["id"], 1);
        assert_eq!(parsed["result"]["protocolVersion"], "2025-03-26");
        assert!(parsed["result"]["capabilities"]["tools"].is_object());
        assert_eq!(parsed["result"]["serverInfo"]["name"], "a2x-mcp-bridge");
    }

    #[test]
    fn test_handle_tools_list() {
        let id = serde_json::Value::Number(2.into());
        let response = handle_tools_list(Some(&id));
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
        let tools = parsed["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 4);
        let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert!(names.contains(&"compile_program"));
        assert!(names.contains(&"validate_program"));
        assert!(names.contains(&"decompile_packet"));
        assert!(names.contains(&"get_info"));
    }

    #[test]
    fn test_handle_tools_call_unknown_tool() {
        let id = serde_json::Value::Number(3.into());
        let params = serde_json::json!({ "name": "nonexistent", "arguments": {} });
        let response = handle_tools_call(Some(&id), &params);
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed["error"]["code"], serde_json::json!(-32601));
    }

    #[test]
    fn test_exec_compile_program_valid() {
        let id = serde_json::Value::Number(4.into());
        let args = serde_json::json!({ "source": "⟦Σ∞⟧⟬I:⚡ ∷ C:⟨sys⟩ ∷ P:⥂ ∷ D:⌬⟭" });
        let response = exec_compile_program(Some(&id), &args);
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
        let text = parsed["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("compilation_success"));
    }

    #[test]
    fn test_exec_compile_program_invalid() {
        let id = serde_json::Value::Number(5.into());
        let args = serde_json::json!({ "source": "not valid sigma" });
        let response = exec_compile_program(Some(&id), &args);
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed["result"]["isError"], true);
    }

    #[test]
    fn test_exec_validate_program_valid() {
        let id = serde_json::Value::Number(6.into());
        let args = serde_json::json!({ "source": "⟦Σ∞⟧⟬I:✦ ∷ C:⟨scope⟩ ∷ P:⥂ ∷ D:⌵⟭" });
        let response = exec_validate_program(Some(&id), &args);
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
        let text = parsed["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("valid"));
    }

    #[test]
    fn test_exec_validate_program_invalid() {
        let id = serde_json::Value::Number(7.into());
        let args = serde_json::json!({ "source": "⟦Σ∞⟧⟬I:⟘ ∷ C:⟘ ∷ P:⟘ ∷ D:⟘⟭" });
        let response = exec_validate_program(Some(&id), &args);
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed["result"]["isError"], true);
    }

    #[test]
    fn test_exec_get_info() {
        let id = serde_json::Value::Number(8.into());
        let response = exec_get_info(Some(&id));
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
        let text = parsed["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("A2X"));
        assert!(text.contains("Agent-to-Anything"));
    }

    #[test]
    fn test_process_message_initialize() {
        let msg = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}"#;
        let response = McpBridge::process_message(msg);
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed["id"], serde_json::json!(1));
        assert_eq!(parsed["result"]["protocolVersion"], "2025-03-26");
    }

    #[test]
    fn test_process_message_notification() {
        let msg = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
        let response = McpBridge::process_message(msg);
        assert!(response.is_empty());
    }

    #[test]
    fn test_process_message_ping() {
        let msg = r#"{"jsonrpc":"2.0","id":9,"method":"ping"}"#;
        let response = McpBridge::process_message(msg);
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed["id"], serde_json::json!(9));
        assert!(parsed["result"].is_object());
    }
}
