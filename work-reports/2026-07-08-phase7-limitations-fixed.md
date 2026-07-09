# Work Report — 2026-07-08 Phase 7 Known Limitations Fixed

## Summary

Addressed the 3 remaining known limitations from the Phase 7 ecosystem hardening work. All fixes compiled and tested (70/71 tests pass, 1 pre-existing port conflict).

## Changes

### 1. Deduplicated Conversation Path

**Problem:** The path `~/.a2x/conversations/chat-1.json` was constructed in two places — `gateway.rs` (for loading) and `dashboard.rs` (for saving) — with identical `std::env::var("HOME")` logic.

**Fix:** Extracted `GatewayState::conversation_path() -> Option<PathBuf>` — returns `None` when HOME is empty (matching the old guard behavior). Both call sites now use this single source of truth.

### 2. Async CCS VM Execution (block_in_place)

**Problem:** `run_ccs_program_impl` used `handle.block_on(...)` directly, which blocks a tokio worker thread for up to 30 seconds during VM execution. While the chat loop wasn't blocked (tool calls run in a spawned task), this starves the async worker pool.

**Fix:** Wrapped `handle.block_on(...)` in `tokio::task::block_in_place(|| ...)`. This tells tokio to move the blocking work off the async worker thread onto a blocking thread, keeping the worker pool free for other tasks.

### 3. VM Region Stats in Dashboard

**Problem:** The dashboard's `handle_vm_command` "region" handler only returned raw `preview` values — no aggregate statistics. The chat tool `vm_region_impl` already had min/max/mean, but the dashboard WebSocket path didn't.

**Fix:** Added `stats: {mean, min, max, sum}` to the dashboard's region response. Computes sum once, reuses for mean.

### Files Changed

| File | Change |
|------|--------|
| `crates/a2x-gateway/src/gateway.rs` | Added `conversation_path()` + `conversation_path_guard()`; updated `get_chat_agent()` |
| `crates/a2x-gateway/src/dashboard.rs` | Use `GatewayState::conversation_path()`; added region stats to `handle_vm_command` |
| `crates/a2x-agents/src/chat_tools.rs` | Wrapped `block_on` in `tokio::task::block_in_place` |

### Verification

- **Build:** `cargo check -p a2x-agents -p a2x-gateway` — pass
- **Tests:** 70/71 pass (1 pre-existing port conflict)
- **Code review:** no blocking issues

## Current System Status

The dashboard now has:
- Live WebSocket snapshots (500ms) with agent cards, WorldGraph force graph, StateField heatmap, VM region heatmap
- Context usage bar (tokens used/max + message count)
- Chat tab with Ollama model listing, model switching, streaming responses, conversation persistence
- VM introspection panel (Status, Belief, Attention, Goal, Trace buttons)
- Bus traffic log, program execution history, timeline
- Σ∞ program playground

ChatAgent features:
- LLM backend (Ollama/OpenAI/Noop) with streaming tool-use loop
- Context memory (Copilot-style: mines dropped messages for file paths, tool usage, topics)
- Sliding window pruning with memory injection
- Conversation save/load to `~/.a2x/conversations/chat-1.json`
- 14 tool definitions (Sigma, CCS VM, CLI, Bus, Omega)
- Shared gateway bus (Arc<Mutex<Bus>>)
- Async CCS VM execution with block_in_place

## Known Limitations (Updated)

- ~~Conversation path duplicated~~ → **Fixed**
- ~~Tool execution blocks tokio worker threads~~ → **Fixed** (block_in_place)
- ~~VM region returns raw data, no stats~~ → **Fixed**
- `execute_tool` is synchronous — tool execution blocks within chat_streaming (minor, only noticeable with long-running VM programs)
- No CLI integration for ChatAgent yet — chat only available through the web dashboard
- WorldGraph starts empty — needs a bootstrap program or pre-loaded concepts
