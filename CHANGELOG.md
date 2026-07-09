# Changelog

All notable changes to the A2X project.

## [v0.9.0-alpha] — 2026-07-08

### Added
- **CLI shell** (`a2x shell`) — interactive Σ∞ REPL with colored output
- **CLI dashboard** (`a2x dashboard`) — one-command gateway launch with browser open
- **CLI monitor** (`a2x monitor`) — bus state viewer with agent listing
- **WorldGraph bootstrap** — 12 concept nodes + 10 relation edges pre-loaded on startup
- **Context usage indicator** — tokens used/remaining bar in Chat tab header
- **Conversation persistence** — auto-save/load chat history to `~/.a2x/conversations/`
- **Ollama model picker** — dropdown in dashboard Chat tab, hot-swap models
- **VM introspection panel** — status, region, trace readout via WebSocket
- **vm_region heatmap** — color-coded StateField region tiles
- **Real WorldGraph data** — dashboard force graph fed from live CCS VM
- **Integration tests** — gateway HTTP execute roundtrip, WorldGraph bootstrap verification

### Fixed
- Chat agent now shares gateway bus (`Arc<Mutex<Bus>>`) instead of isolated bus
- CCS VM execution uses `block_in_place` to avoid tokio worker starvation
- VM region response includes min/max/mean/sum stats
- Deduplicated conversation path construction
- Port conflict test uses port 0 instead of hardcoded 8778
- `HttpListener::bound_address()` returns actual resolved address

### Changed
- `ContextMemory` extracted to own module with 23 edge-case unit tests
- CLI depends on `a2x-gateway` directly for dashboard command

## [v0.8.0] — 2026-07-06

### Added
- **Web dashboard** — single-page app with WebSocket streaming (500ms snapshots)
- Dashboard features: agent cards, WorldGraph force graph, StateField heatmap, bus traffic log
- Σ∞ program playground with execute/result panel
- Chat tab with streaming LLM responses

## [v0.7.1] — 2026-07-06

### Added
- **ChatAgent** — conversational agent with Ollama/OpenAI backend, 14 tool definitions
- **Context memory** — Copilot-style: mines dropped messages for file paths, tool usage, topics
- Sliding window history pruning with memory injection
- Tool execution: Sigma, CCS VM, CLI, Bus, Omega subsystems
- Streaming chat responses via WebSocket

### Fixed
- P1 bus hardening: duplicate agent detection, online/offline lifecycle, discovery dedup
- Boot sequence gap audit across all 9 crates
- BusBridge convenience wrapper for domain event publishing

## [v0.7.0] — 2026-07-05

### Added
- **CCS VM Fork/Merge** — parallel swarm execution with child VM snapshots
- **Bus Transport refactoring** — Bus and BusBridge generic over `T: Transport`
- TcpAsyncBridge + AgentIdentity E2E signing pipeline
- TlsTransport for encrypted bus communication

## [v0.6.0] — 2026-06-28

### Added
- **Phase 6 — Entity Gateway**: Gateway service with HTTP/WS/TCP/stdio listeners
- Entity authentication (API key, JWT, local)
- Rate limiting (token bucket per entity)
- Webhook callback system
- `a2x-client` Rust SDK for external apps
- All 10 crates scaffolding complete

### Changed
- 29 audit roadmap items completed across all tiers
- Comprehensive bugfix round — parser, tokenizer, VM edge cases
- All clippy warnings resolved

## [v0.5.0] — 2026-06-28

### Added
- **Phase 5 — Probe & Debug**: probe protocol, breakpoints (instruction/opcode/conditional)
- Instruction tracer with configurable verbosity modes
- MemoryTrace timeline viewer
- WorldGraph inspector

## [v0.4.0] — 2026-06-28

### Added
- **Phase 4 — Training & Learning**: learned encoder/decoder (neural network)
- Training loop in simulated environments
- `candle` integration for GPU-accelerated training

## [v0.3.0] — 2026-06-28

### Added
- **Phase 3 — Ω Latent Protocol**: OmegaPacket with const-generic dimension
- Compiler pipeline (7 stages): lexer → parser → semantic → IR → optimizer → codegen → serializer
- Optimizer passes: constant folding, dead code elimination, instruction fusion, layout optimization
- Binary wire format with CRC32 checksum
- Σ∞ ↔ Ω bridge
- TCP transport layer, cross-machine agent communication

## [v0.2.0] — 2026-06-28

### Added
- **Phase 2 — CCS Cognitive Substrate**: WorldGraph (petgraph-backed), StateField (ndarray)
- All 7 VM operators: Bind, Differentiate, Ground, Evolve, Reflect, Plan, Actuate
- MemoryTrace with RLE + hash-dedupe compression
- Fork/Merge for parallel swarm execution
- CCS agent with cognitive tick loop
- VM step-by-step execution, call stack, control flow (Jump, Branch, Call, Return)
- Phase 2 smoke test exercising every operator

## [v0.1.0] — 2026-06-28

### Added
- **Phase 1 — Σ∞ Protocol Core**: tokenizer, parser, serializer
- All operator tables: Intent (11 ops), Context (11 ops), Plan (12 ops), Data (11 ops)
- SigmaProgram — sequences of packets forming programs
- Binary encoding: 1-byte header + 4-byte operand + 2-byte control + variable data
- Property-based tests (proptest roundtrip), fuzz targets for tokenizer/parser
- Criterion benchmarks for tokenizer throughput

## [v0.0.0] — 2026-06-28

### Added
- **Phase 0 — Scaffold & Core**: Cargo workspace with 10 crates
- `a2x-core`: ConceptVector, RelationEdge, RelationType, WorldGraph trait, StateField, Agent trait, Error types
- CI pipeline (GitHub Actions): lint, build, test, benchmark on ubuntu + windows
- Unified SemVer across all crates
- Pre-commit hooks: `cargo fmt`, `cargo clippy`, `cargo test`
