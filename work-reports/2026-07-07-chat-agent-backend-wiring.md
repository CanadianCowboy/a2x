# Work Report — 2026-07-07 Chat Agent Backend Wiring

## Summary

Implemented a full **ChatAgent** — a conversational A2X coding agent accessible through the web dashboard, powered by local LLMs (Ollama) with real-time streaming and unrestricted PC access. The agent wields all A2X subsystems (Sigma, Omega, CCS, Bus, Probe, CLI) through natural language.

## Changes

### New Files

- `crates/a2x-agents/src/chat_agent.rs` — ChatAgent with tool-use loop, conversation history, streaming support, stats
- `crates/a2x-agents/src/chat_tools.rs` — 10 tool definitions wrapping all A2X subsystems (execute_sigma, parse_sigma, list_agents, probe_agent, inspect_graph, shell_exec, fs_read, fs_write, run_ccs_program, compile_omega)
- `crates/a2x-agents/src/chat_prompt.rs` — System prompt describing all A2X subsystems for the LLM, ReAct fallback parser

### Modified Files

**Core types:**
- `crates/a2x-core/src/agent_id.rs` — Added `AgentType::Chat` variant
- `crates/a2x-core/src/capability.rs` — Added `Chat`, `Generate`, `Reflect` capabilities

**LLM backend:**
- `crates/a2x-agents/src/llm_backend.rs` — Added `ChatChunk`, `ChatMessage`, `ChatRole`, `ToolCall`, `ToolDef` types + `chat_stream()` streaming method with SSE parsing for OpenAI-compatible APIs (OpenAI, Ollama)
- `crates/a2x-agents/src/lib.rs` — Exported all new modules and types

**Gateway:**
- `crates/a2x-gateway/src/gateway.rs` — Added `ChatAgent` to `GatewayState` with lazy init; backend selection from config (ollama/openai/none); CliAgent uses `SandboxMode::None` for unrestricted access; Chat agent registered as builtin
- `crates/a2x-gateway/src/config.rs` — Added `ChatBackendConfig` struct with ollama/openai/none modes, model, API URL, temperature. Fixed `GatewayConfig::default()` to include it
- `crates/a2x-gateway/src/bin/a2x-gatewayd.rs` — Reads `A2X_CHAT_BACKEND`, `A2X_CHAT_MODEL`, `A2X_CHAT_API_URL`, `A2X_CHAT_API_KEY` env vars; eagerly validates backend at startup; prints configured backend
- `crates/a2x-gateway/src/listeners/http.rs` — Records execution results in dashboard history

**Dashboard:**
- `crates/a2x-gateway/src/dashboard.rs` — Major updates:
  - Chat tab in the dashboard with streaming message bubbles, code formatting, loading indicator
  - WebSocket `chat`/`chat_response`/`chat_done`/`chat_error` message protocol
  - Replaced blocking `chat()` with async `chat_streaming()` via mpsc channel bridge for real-time token-by-token streaming
  - Fixed MutexGuard Send issue (Result<MutexGuard> temporary held across .await)
  - Inline tool call indicators in streaming bubble (🔧 Running: tool_name / ✓/✗ results)
  - requestAnimationFrame throttling for O(1) frame cost on re-renders
  - Tool indicator CSS styling (.tool-indicator.ok/.err)

**Dependencies:**
- `Cargo.toml` — Added `futures-util` workspace dep
- `crates/a2x-agents/Cargo.toml` — Added `futures-util`, `a2x-core` deps
- `crates/a2x-gateway/Cargo.toml` — Added `a2x-ccs` dependency

## Architecture

```
Dashboard Chat Tab
    │  WebSocket
    ▼
handle_chat_message()
    │  mpsc channel
    ├─► tokio::spawn(chat_streaming)  ──► LLM (Ollama/OpenAI)
    │       │                                   │
    │       ▼ SSE streaming                     │ tool calls
    │   on_chunk callback ◄─────────────────────┘
    │       │
    ▼       ▼
    rx.recv().await ──► socket.send()  ──► Browser (real-time rendering)
```

### Tool-Use Loop
User message → LLM (with tools) → tool calls execute → results fed back → LLM responds → text streamed to user

### Tools
| Tool | A2X Subsystem | Purpose |
|------|--------------|---------|
| `execute_sigma` | Σ∞ | Execute Sigma programs |
| `parse_sigma` | Σ∞ | Parse/validate syntax |
| `list_agents` | Bus | Discover agents |
| `probe_agent` | Probe | Inspect agent state |
| `inspect_graph` | CCS | Examine WorldGraph |
| `shell_exec` | CLI | Run any shell command |
| `fs_read` | CLI | Read files |
| `fs_write` | CLI | Write files |
| `run_ccs_program` | CCS | Cognitive operations |
| `compile_omega` | Ω | Compile to latent form |

## Verification

- **Build:** `cargo check -p a2x-agents -p a2x-gateway -p a2x-core` — all pass
- **Tests:** 69/70 pass (1 pre-existing port conflict failure in `test_http_listener_lifecycle`)
- **ChatAgent tests:** `test_new_chat_agent`, `test_history_starts_with_system_prompts`, `test_chat_adds_to_history`, `test_capabilities`, `test_state_summary`, `test_reset_history`, `test_compact_constructor`, `test_stats` — all pass

## Usage

```bash
# Start with Ollama backend
A2X_CHAT_BACKEND=ollama A2X_CHAT_MODEL=deepseek-coder-v2:latest cargo run -p a2x-gateway

# Available models on this machine
# deepseek-coder-v2:latest, deepseek-coder:1.3b, qwen2.5:7b, mistral:latest
```

Open `http://localhost:8778`, switch to the **Chat** tab, and start chatting.

## Known Limitations

- Chat agent uses an isolated bus (can't discover gateway-registered agents)
- Tool call visibility: inline indicators appear only after text streaming starts
- No conversation persistence across restarts
- Work report file not yet created (this is it)

## Next Steps

- Wire dashboard model picker UI to switch between downloaded models
- Share the gateway bus with the ChatAgent for full agent discovery
- Persist conversations to `~/.a2x/conversations/`
