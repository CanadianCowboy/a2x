# A2X — Functional Gaps and Minimal Boot Plan (2026-07-01)

This note distills the functionality-critical gaps for an end-to-end run and provides a minimal “get it working” plan. It complements the broader audit in `work-reports/2026-07-01-audit-findings.md`.

## What’s needed for actual functionality
- Service entrypoint to expose the HTTP API and serve routes.
- Wiring to a basic execution path for programs (Sigma/Ω) without requiring external infra.
- Simple configuration defaults and a way to run locally with one command.
- Basic logs and a way to keep the process alive.

What’s already present and leveraged here:
- Gateway HTTP listener and handlers (Axum) with `POST /a2x/execute`, entity listing, probing, and webhook registration are implemented in `crates/a2x-gateway`.
- Program execution uses an in-process orchestrator (`a2x-agents::Orchestrator`) via `GatewayState::execute_program`, so an external worker is not required for a minimal demo.

## Minimal boot plan (today)
1) Add a small gateway daemon binary (`a2x-gatewayd`) inside `a2x-gateway` that:
   - Reads `A2X_HTTP_ADDR` (host:port) or falls back to `A2X_HTTP_PORT` (default `8778`).
   - Builds `GatewayConfig` from env, optionally registering `A2X_API_KEY` for API-key auth.
   - Starts the existing `HttpListener` on the chosen address and serves the REST API.
   - Provides `/healthz` and `/readyz`; blocks the main thread so it keeps serving.
2) Provide a curl example to verify `POST /a2x/execute` returns a result.

## How to run (local)
- Build the workspace:
  ```bash
  cargo build
  ```
- Start the gateway daemon (default port 8778):
  ```bash
  cargo run -p a2x-gateway --bin a2x-gatewayd
  # or change the port
  A2X_HTTP_PORT=8888 cargo run -p a2x-gateway --bin a2x-gatewayd
  # or bind explicitly (overrides port)
  A2X_HTTP_ADDR=127.0.0.1:9001 cargo run -p a2x-gateway --bin a2x-gatewayd
  ```
- Call the execute endpoint from another terminal:
  ```bash
  curl -sS -X POST \
       -H "Content-Type: application/json" \
       -d '{"program":"⟦Σ∞⟧⟬I:✕⟭","format":"sigma","timeout_ms":1000}' \
       http://127.0.0.1:8778/a2x/execute | jq .
  ```
  Expected shape:
  ```json
  { "result": "...", "execution_time_ms": 12, "status": "completed" }
  ```

- Health/readiness checks:
  ```bash
  curl -sS http://127.0.0.1:8778/healthz
  curl -sS http://127.0.0.1:8778/readyz
  ```

- With API key (optional):
  ```bash
  # Start with an API key
  A2X_API_KEY=sk-local-123 cargo run -p a2x-gateway --bin a2x-gatewayd
  # Then call with the key
  curl -sS 'http://127.0.0.1:8778/a2x/execute?api_key=sk-local-123' \
       -H 'Content-Type: application/json' \
       -d '{"program":"⟦Σ∞⟧⟬I:✕⟭"}' | jq .
  ```

## Next steps (beyond minimal)
- Add structured tracing output (JSON) and basic metrics.
- Introduce real auth verification (JWT/API key) for internet-facing use.
- Add a separate agent worker binary and connect via the bus for multi-process topologies.
- Provide a docker-compose example to run gateway + worker.

## Evidence pointers
- HTTP API and listener lifecycle: `crates/a2x-gateway/src/listeners/http.rs`
- Program execution path: `crates/a2x-gateway/src/gateway.rs` (`execute_program`, `execute_program_for_entity`)
