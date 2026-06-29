# Phase 6 — Entity Integration (Gateway + Client SDK)

> **Date:** 2026-06-28
> **Tag:** v0.6.0 (commit `290fbe9`)
> **Scope:** plans/06-entity-gateway.md, PLAN.md §30

---

## Summary

Phase 6 implements the Entity Gateway — the bridge between external entities (apps, APIs, humans, systems) and the A2X ecosystem. This is the "Anything" in Agent-to-Anything.

---

## Deliverables

### 6.1 Core Types (a2x-gateway)

| Type | Description |
|------|-------------|
| `EntityId` | Newtype over String, distinct from AgentId |
| `Entity` trait | entity_id, entity_type, display_name, is_alive, capabilities |
| `EntityInfo` | Serializable entity metadata for discovery |
| `SimpleEntity` | Test/config entity backed by metadata |
| `EntityType` | Re-exported from a2x-core (HumanCli, Application, etc.) |
| `Capability` | Re-exported from a2x-core |

### 6.2 Authentication (a2x-gateway/auth.rs)

| Type | Description |
|------|-------------|
| `AuthMethod` | ApiKey, BearerToken, Local |
| `EntityPermissions` | max_instructions, can_probe, can_network, rate_limit |
| `AuthProvider` trait | authenticate → EntityId, permissions → EntityPermissions |
| `InMemoryAuthProvider` | API key store with permission lookup |

### 6.3 Configuration (a2x-gateway/config.rs)

TOML-deserializable `GatewayConfig` with sections for HTTP, WebSocket, TCP, stdio, auth (API keys), and webhooks. All fields have sensible defaults.

### 6.4 Gateway Service (a2x-gateway/gateway.rs)

| Method | Purpose |
|--------|---------|
| `Gateway::new()` | Default gateway with in-memory bus |
| `Gateway::from_config()` | Parse TOML config, register API keys |
| `GatewayState::register_entity()` | Add entity to registry |
| `GatewayState::execute_program()` | Route Σ∞ to orchestrator via bus |
| `GatewayState::probe_agent()` | Try all 4 agent types, return state |
| `GatewayState::authenticate()` | Validate auth method |
| `Gateway::start()` / `stop()` | Lifecycle management |
| `register_builtin_agents()` | 4 demo agents (orch, cli, llm, ccs) |

### 6.5 Protocol Listeners

| Listener | Protocol | Key Features |
|----------|----------|-------------|
| `HttpListener` | HTTP/REST | axum-based, 5 endpoints (/execute, /entities, /entities/:id, /probe/:id, /webhook) |
| `WebSocketListener` | WebSocket | Text frames (Σ∞) + binary frames (Ω, 4-byte length prefix) |
| `TcpListener` | Raw TCP | 4-byte BE length-prefix framing (matches a2x-bus format) |
| `StdioListener` | stdin/stdout | Line-by-line Σ∞ processing, comment support, hex binary output |

### 6.6 Webhook System (a2x-gateway/webhook.rs)

`WebhookManager` with register/unregister, entity-scoped cleanup, correlation ID filtering, and JSON payload preparation.

### 6.7 Client SDK (a2x-client)

| Method | Purpose |
|--------|---------|
| `A2xClient::new()` | Connect to gateway URL with API key |
| `execute()` | POST /a2x/execute — run Σ∞ program |
| `list_entities()` | GET /a2x/entities — discover entities |
| `probe_agent()` | GET /a2x/probe/:id — inspect agent state |
| `register_webhook()` | POST /a2x/webhook — async result callback |

`ClientError` enum: Network, ServerError, ParseError.

### 6.8 Workspace Updates

- Added `axum`, `tower-http`, `toml`, `reqwest`, `uuid`, `serde_json` to workspace deps
- a2x-gateway Cargo.toml: 13 dependencies
- a2x-client Cargo.toml: 8 dependencies

---

## Test Coverage

| Crate | Tests | Status |
|-------|-------|--------|
| a2x-gateway | 52 | ✅ All pass |
| a2x-client | 6 (1 doc-test) | ✅ All pass |
| **Total new** | **59** | ✅ |

**Clippy:** Clean (0 warnings)
**Fmt:** Clean

---

## Files Created/Modified

| File | Purpose |
|------|---------|
| `Cargo.toml` | +6 workspace deps (axum, tower-http, toml, reqwest, uuid, serde_json) |
| `crates/a2x-gateway/Cargo.toml` | Gateway crate manifest |
| `crates/a2x-gateway/src/lib.rs` | Module declarations + re-exports |
| `crates/a2x-gateway/src/error.rs` | GatewayError enum (10 variants) |
| `crates/a2x-gateway/src/entity.rs` | Entity trait, EntityId, EntityInfo, SimpleEntity |
| `crates/a2x-gateway/src/auth.rs` | AuthMethod, EntityPermissions, AuthProvider |
| `crates/a2x-gateway/src/config.rs` | GatewayConfig (TOML), all sub-configs |
| `crates/a2x-gateway/src/webhook.rs` | WebhookManager with filtering |
| `crates/a2x-gateway/src/gateway.rs` | GatewayState + Gateway service |
| `crates/a2x-gateway/src/listeners/mod.rs` | ProtocolListener trait, message types |
| `crates/a2x-gateway/src/listeners/http.rs` | axum HTTP listener + handlers |
| `crates/a2x-gateway/src/listeners/ws.rs` | WebSocket frame parsing |
| `crates/a2x-gateway/src/listeners/tcp.rs` | TCP length-prefix framing |
| `crates/a2x-gateway/src/listeners/stdio.rs` | stdin/stdout line processing |
| `crates/a2x-client/Cargo.toml` | Client crate manifest |
| `crates/a2x-client/src/lib.rs` | A2xClient SDK |

---

## Plan Compliance

| Section | Deliverable | Status |
|---------|-------------|--------|
| §1 | Gateway overview | ✅ |
| §2 | Architecture diagram | ✅ (implemented) |
| §3 | Entity trait | ✅ |
| §4 | Gateway service | ✅ |
| §5 | HTTP listener | ✅ (5 endpoints) |
| §5 | WebSocket listener | ✅ (text + binary) |
| §5 | TCP listener | ✅ (length-prefix) |
| §5 | stdio listener | ✅ (line-by-line) |
| §5 | Webhook callback | ✅ |
| §6 | Authentication | ✅ (API key + JWT stub + local) |
| §7 | Client SDK | ✅ (Rust) |
| §8 | Configuration | ✅ (TOML) |

### Reviewer Notes (deferred to Phase 7+):

- `std::sync::Mutex` in async HTTP handlers → switch to `tokio::sync::Mutex`
- `Gateway::add_listener()` not yet public → wire listener registration
- `EntityPermissions` not enforced in auth flow → add enforcement layer
- `incoming_tx`/`response_rx` fields in WS/TCP listeners are scaffolding → wire into listener logic

---

## Next Steps

- Wire tokio async runtime into HTTP/WS listeners (tokio::sync::Mutex)
- Add `Gateway::add_listener()` for dynamic listener registration
- End-to-end demo: HTTP client → gateway → bus → agent → result
- Python/JavaScript client SDKs (third-party)
- Entity protocol listener crates (a2x-entity-http, etc.)

---

*This file is part of the A2X project. See PLAN.md for the full architecture.*
