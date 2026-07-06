# A2X - Project-wide Audit Findings and Recommendations (2026-07-01)

This report summarizes verified fixes, gaps, risks, and concrete improvements across the repository as of 2026-07-01. It is evidence-backed with code paths and keeps scope to what was directly inspected.

## Scope and method
- Read top-level docs: `README.md`, `PLAN.md`
- Inspected workspace metadata: `Cargo.toml`
- Targeted code review of key crates to validate claims and find gaps
  - Bus codec and transport
  - Gateway auth and permission enforcement
  - Agents (LLM backend integration)
  - Probe tools
- Skipped running tests/build (documentation-only audit); recommend CI to verify counts automatically

## Executive summary
- Several previously reported issues appear fixed (bus decode safety, gateway permission enforcement, LLM agent backend abstraction).
- Documentation and metadata are inconsistent with the codebase (version, crate count, “no code exists yet”).
- Essentials for production-readiness (CI, dynamic test badges, stronger auth verification, release process) need attention.
- Clear quick wins are available with low effort and high clarity impact.

## Verified improvements since prior audits
- Bus TCP frame decoding is error-safe (no `unwrap()` in library paths)
  - See `crates/a2x-bus/src/tcp_transport.rs` function `decode_message()` returning `Result<_, TransportError>` and unit tests at bottom of the file that assert error paths on truncated frames.
- Gateway now enforces permissions and rate limits
  - See `crates/a2x-gateway/src/gateway.rs` methods `execute_with_permissions`, `enforce_permissions`, and `enforce_rate_limit`, plus HTTP listener returning 401 for bad API key in `crates/a2x-gateway/src/listeners/http.rs`.
- LLM agent uses a real, pluggable backend
  - See `crates/a2x-agents/src/llm_agent.rs` (uses a backend trait) and `crates/a2x-agents/src/llm_backend.rs` (OpenAI-compatible backend implementation and prompts). Includes async methods and a no-op backend for testing.
- Probe tools crate present with usable surface
  - See `crates/a2x-probe/src/lib.rs` exposing `ProbeTool` and submodules `inspector` and `tracer` referencing plan docs.

## Documentation and metadata inconsistencies
- Version mismatch
  - README states: “Current version: v0.6.0 (499 tests, all 10 crates passing)”
  - Workspace metadata: `Cargo.toml` has `[workspace.package] version = "0.1.0"`
- Crate count mismatch
  - README “Official Crates” table lists 10 crates; workspace members list 11 (includes `a2x-startup`).
- Outdated onboarding note
  - README section “If You Are Starting Fresh” still says “No code exists yet”, which is now incorrect.
- Test count claim likely stale
  - README claims “499 tests”; no CI badge or automated tally present; number likely outdated.

## Current state by crate (spot-checked)
- a2x-bus
  - Wire framing and decode paths are robust against truncation and malformed input; round-trip tests exist.
  - TLS and identity modules are present; further review of key handling and error propagation is advised before production use.
- a2x-gateway
  - Auth provider and permission model exist; HTTP listener rejects invalid credentials; rate-limiting implemented.
  - Recommend stronger JWT verification in production (currently structural only), with key material and proper validation flow.
- a2x-agents
  - LLM agent integrates a real backend abstraction; OpenAI-compatible client included with prompts to translate NL <-> Sigma programs.
  - Provides synchronous wrappers that require a Tokio runtime; consider full async surfaces to avoid blocking contexts.
- a2x-probe
  - Tooling surfaces exist (snapshot, breakpoints, tracer mode). Good foundation for developer ergonomics.
- a2x-startup
  - Crate exists for startup/shutdown/config; includes optional key-rotation feature flags; recommend documenting usage and integration points.
- Other crates (core, sigma, omega, ccs, client, cli)
  - Not reviewed in depth in this pass; README and prior plans indicate mature scaffolding. Recommend CI to continuously validate.

## Missing or partial features (priority items)
1) CI/CD and automated verification
- Add GitHub Actions (or preferred CI) to run `cargo clippy`, `cargo test --all`, and to publish a dynamic test count badge.
- Add minimal smoke integration tests for cross-crate flows (bus <-> gateway <-> agents) to catch regressions.

2) Versioning and release process
- Align README version with workspace version, or bump the workspace per the roadmap.
- Add a RELEASE.md and unify tagging across all crates (workspace versioning strategy already noted in README decisions).

3) Auth and security hardening
- JWT: move from structural checks to cryptographic verification with configured JWKS or HMAC/EdDSA keys.
- Rate limiter: add tests for burst/steady-state behavior to prove limits and refill logic.
- Document production guidance: TLS termination, key rotation (see `a2x-startup`), secrets management.

4) Observability and ops
- Ensure `tracing` subscriber initialization in binaries with sane defaults; add log fields for entity_id, program_id, correlation_id.
- Consider metrics (e.g., Prometheus) for request rates, error counts, and queue depths (bus/gateway).

5) Developer experience
- Add a Quickstart in README for running an end-to-end example (Gateway HTTP listener + simple agent + client request), with exact commands.
- Document `a2x-startup` usage and sample config TOML under `docs/`.

6) Testing depth
- Add property/fuzz tests for parsers and wire formats (if not already present in sigma/bus) and serialization round-trips.
- Add concurrency tests for async bus/VM paths (Phase 7 work-in-progress per README).

## Risks and watch-outs
- Blocking calls in LLM agent through a current Tokio handle can deadlock in some runtimes; prefer an async-only API surface or spawn-blocking with care.
- JWT structural-only checks are not sufficient for multi-tenant or internet-facing deployments.
- Version/documentation drift harms contributor onboarding and trust in test counts.

## Quick wins (low effort, high impact)
- Fix README inconsistencies (version, crate count, onboarding note) and add a Quickstart example.
- Add a GitHub Actions workflow for `cargo fmt --check`, `clippy`, and `test --all`; publish a test-count badge rendered from CI output.
- Add `RELEASE.md` with the unified versioning policy outlined in README and automate tagging with a simple script.

## Near-term plan (1–2 weeks)
- Harden gateway auth: implement JWT verification with configurable keys; add permission enforcement tests for can_probe and rate limit edge cases.
- Make LLM agent surfaces fully async; provide an example that uses a real local endpoint (e.g., Ollama) without blocking.
- Add basic integration tests spanning bus <-> gateway <-> agent request execution.

## Mid-term (1–2 months)
- Expand observability (metrics, trace IDs across bus and gateway, correlation IDs end-to-end).
- Document and demo `a2x-startup` (config, key rotation, persistence), and wire it into CLI/gateway examples.
- Broaden backend support (e.g., Anthropic-compatible) via the existing trait, including retry/backoff and timeouts.

## Evidence index (selected)
- README version and onboarding text: `README.md` (sections: Project Status; If You Are Starting Fresh)
- Workspace version and members: `Cargo.toml` `[workspace]` and `[workspace.package]`
- Bus codec safety: `crates/a2x-bus/src/tcp_transport.rs` (`decode_message`, tests)
- Gateway permission enforcement: `crates/a2x-gateway/src/gateway.rs` (`enforce_permissions`, `enforce_rate_limit`) and `crates/a2x-gateway/src/listeners/http.rs`
- LLM backend integration: `crates/a2x-agents/src/llm_agent.rs`, `crates/a2x-agents/src/llm_backend.rs`
- Probe tooling: `crates/a2x-probe/src/lib.rs`

## Notes
- This audit focused on correctness of claims and safety of critical paths; it did not count tests precisely nor execute builds. CI is recommended to provide authoritative, always-fresh counts and status.
- If you want this report expanded with deeper per-crate coverage, I can extend the pass to sigma/omega/ccs internals and add targeted recommendations there.