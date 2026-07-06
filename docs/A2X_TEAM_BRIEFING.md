# A2X Team Briefing — Decisions from Soong Path

> **To:** a2x Development Team
> **From:** Josh (Project Lead — Soong Path & a2x)
> **Date:** 2026-07-05
> **Purpose:** Inform a2x developers of architectural decisions made in Soong Path that affect a2x's roadmap, integration points, and future direction.

---

## 1. Background — Why a2x Exists

a2x was built as a utility framework to make Soong Path development easier in the long run. It provides the communication layer (bus), symbolic ISA (Σ∞), latent compiler (Ω), cognitive VM (CCS), agent orchestration, gateway, and debugging — all things Soong Path needs.

**Soong Path is the passion project. a2x is the tool that makes it possible.**

This doesn't mean a2x is less important — it means a2x has a different future:
- **Soong Path** stays personal. One system, ever-evolving. Not handed off.
- **a2x** is hand-offable. Other developers can take over a2x development. a2x can outgrow Soong and serve other projects too.

---

## 2. Key Decisions That Affect a2x

### 2.1 Soong OS Will Host the a2x Bus (Interim)

**Decision:** Soong OS will run the a2x bus host as a system service (PID managed by soong-init). This is the **interim** arrangement during development and early operation.

**What this means for a2x:**
- The `a2x-bus` crate's `InMemoryAsyncBus` + `TcpAsyncBridge` will be used as a standalone service inside Soong OS
- The bus binds to port 8420 and accepts external TCP/TLS connections
- Soong's `soongd` daemon connects to the bus in-process (zero latency)
- External a2x agents connect over the network via TCP/TLS

**What a2x needs to support:**
- `BusBridge` must work as the sole interface between soongd and the bus (no direct `InMemoryAsyncBus` calls from soongd)
- TCP transport (`TcpAsyncBridge`) must be production-ready for external agent connections
- TLS transport (`TlsTransport`) must support mTLS for authenticated external agents
- Ed25519 identity signing must work for agent authentication

### 2.2 a2x Will Graduate to Its Own Host (Future)

**Decision:** When a2x has its own funding and infrastructure, the bus host will migrate from Soong OS to a dedicated a2x service. Soong OS will then connect TO the external bus as a participant.

**What this means for a2x:**
- a2x will eventually run its own `a2x-bus-host` on dedicated infrastructure
- Soong OS will connect to it remotely via `TcpAsyncBridge`
- The `Bus` struct is currently hardcoded to `InMemoryTransport` — **this needs to be refactored to accept any `Transport` trait implementation** (in-memory or TCP) before graduation
- The `BusBridge` API should remain stable so Soong OS's code doesn't change during migration

**Graduation prerequisites (a2x side):**
1. `Bus` struct refactored to accept generic `Transport` trait (not hardcoded to `InMemoryTransport`)
2. Dedicated a2x infrastructure provisioned
3. `a2x-bus-host` running with proper TLS CA
4. Soong OS's Ed25519 identity registered on the new bus host

### 2.3 Soong OS's a2x Dependency Is Permanent

**Decision:** Soong OS will **always** use a2x for all agent communication — before, during, and after bus graduation. This never changes.

**What this means for a2x:**
- a2x has a guaranteed long-term user (Soong Path)
- The `BusBridge`, Σ∞ encoding, wire protocol, and agent discovery APIs are Soong's permanent communication layer
- Breaking changes to these APIs affect Soong Path directly
- a2x can and should serve other projects too, but Soong Path is the primary consumer

### 2.4 Soong Path Gap Reports — What a2x Needs to Deliver

Soong Path has identified 9 integration gaps. Here's what a2x needs to deliver to close them:

| # | Gap | Severity | What a2x Needs to Provide |
|:-:|-----|:--------:|---------------------------|
| 01 | CCS VM not running | 🔴 Critical | A working CCS VM that executes Σ∞ programs. Soong's cognitive pipeline needs this. |
| 02 | Core type system not unified | 🟠 High | `ConceptVector`, `WorldGraph`, `StateField`, `NodeId` — Soong wants to use these instead of maintaining parallel types. |
| 03 | Sigma encoding not wired | 🟠 High | Σ∞ encoding API that Soong's perception and cognition crates can call to encode events as packets. |
| 04 | Bus not configured | 🟠 High | ✅ Being resolved — Soong OS will host the bus as a system service. |
| 05 | Agent orchestration not wired | 🟠 High | Agent lifecycle, safety model, capability routing — Soong wants to register as a CCS agent and delegate tasks. |
| 06 | Gateway not active | 🟡 Medium | Gateway service that bridges external entities (web, DB, robots) to the bus. |
| 07 | Omega compilation not used | 🟡 Medium | Ω latent compiler for Soong's pattern/prediction compilation. |
| 08 | Startup boot stub | 🟡 Medium | `BootSequence`, `ConfigLoader`, `RecoveryManager` APIs for ordered initialization. |
| 09 | Probe not active | 🟢 Low | Debug/introspection for cognitive state. Nice to have. |

**Priority order:** #01 (CCS VM) → #02 (core types) → #03 (sigma) → #05 (agents) → #06 (gateway) → #07 (omega) → #08 (startup) → #09 (probe)

---

## 3. What's Already Built and Working (a2x side)

| Component | Status | Tests |
|-----------|--------|:-----:|
| `a2x-bus` — InMemoryAsyncBus, TcpAsyncBridge, BusBridge | ✅ Built | 60 tests |
| `a2x-bus` — TlsTransport, AgentIdentity (Ed25519) | ✅ Built (feature-gated) | — |
| `a2x-bus` — Router, Discovery, WireMessage protocol | ✅ Built | — |
| `a2x-core` — AgentId, ConceptVector, WorldGraph, StateField, NodeId | ✅ Types defined | — |
| `a2x-sigma` — Σ∞ ISA | ✅ Built | — |
| `a2x-ccs` — CryoCore VM | 🟡 Stub | — |
| `a2x-omega` — Latent compiler | 🟡 Stub | — |
| `a2x-gateway` — Entity bridge | 🟡 Stub | — |
| `a2x-startup` — Boot sequence | 🟡 Stub | — |
| `a2x-agents` — Agent orchestration | 🟡 Stub | — |
| `a2x-probe` — Debug/introspection | 🟡 Stub | — |

---

## 4. Soong Path's Integration Plan (How a2x Gets Wired In)

Soong Path's integration roadmap (from Plan 06 and Plan 25):

```
Phase 2 (Boot/ISO):
  → soong-bus-host service added to Soong OS ISO
  → Bus runs from boot as first system service

Phase 3 (Security):
  → Seccomp filter for bus host (network + IPC syscalls only)
  → TLS cert bootstrap
  → Ed25519 agent authentication

Post-Phase 5 (Language):
  → BusBridge wired into soongd's EventStream
  → Events published to bus as Σ∞ packets
  → External agents can subscribe to Soong's cognitive events

Future (post-funding):
  → a2x bus graduates to dedicated infrastructure
  → Soong OS connects to remote bus as participant
```

---

## 5. What a2x Developers Should Know About Soong Path

| Topic | Detail |
|-------|--------|
| **Soong Path is one system** | There is one Soong OS instance, ever. It's not deployed across machines. It's an AGI mind that grows over time. |
| **Soong OS runs on bare metal** | Linux 7.1.0 kernel built from source, btrfs root filesystem, Rust init system (no systemd). |
| **Soong's EventStream** | Internal pub/sub system (tokio broadcast channel). The `BusBridge` translates these to/from Σ∞ packets on the a2x bus. |
| **Soong's cognitive pipeline** | 21-stage pipeline: attention → habituation → curiosity → prediction → causal → emotion → memory → consciousness. All in Rust. |
| **Soong's neural core (SONN)** | Online-learning neural network (candle-based). Learns per-event, continuously. Not a pre-trained model. |
| **Soong's perception** | OpenCV (vision), cpal/JACK (audio). Currently stubs — hardware integration is Phase 1. |
| **Soong's communication** | All agent communication goes through a2x. This is permanent. The bus is Soong's nervous system. |

---

## 6. Action Items for a2x Team

| Priority | Action | Why |
|:--------:|--------|-----|
| 🔴 P0 | Get CCS VM running and executing Σ∞ programs | Soong's cognitive pipeline cannot run a2x programs without it |
| 🔴 P0 | Refactor `Bus` to accept generic `Transport` trait | Prerequisite for bus graduation (in-process → remote migration) |
| 🟠 P1 | Ensure `BusBridge` API is stable and well-documented | Soong Path depends on this as its sole bus interface |
| 🟠 P1 | Productionize `TcpAsyncBridge` for external agent connections | Soong OS will expose the bus on port 8420 to external agents |
| 🟠 P1 | Verify `TlsTransport` + `AgentIdentity` (Ed25519) work end-to-end | Soong OS needs authenticated external agent connections |
| 🟡 P2 | Implement `a2x-gateway` service | External entity bridging (web dashboards, databases, robots) |
| 🟡 P2 | Implement `a2x-startup` boot sequence APIs | Ordered initialization for Soong OS services |
| 🟢 P3 | Implement `a2x-probe` introspection | Debug/observability for Soong's cognitive state |

---

## 7. Cross-References

| Document | Location | Description |
|----------|----------|-------------|
| Plan 25 — Soong OS as A2X Bus Host | `plans/25-soong-os-bus-host.md` | Full architecture for bus host in Soong OS, migration path, graduation checklist |
| Plan 06 — A2X Integration | `soong-path/plans/06-a2x-integration.md` | Dependency matrix, integration status, gap report index |
| Plan 22 — Master Implementation Plan | `plans/22-master-implementation-plan.md` | All Soong Path decisions (22 locked in) |
| A2X Gap Reports | `docs/a2x_gap_reports/` | 9 detailed gap reports with proposed changes |
| Build VM Info | `docs/BUILD_VM_INFO.md` | Linux build VM reference for compiling Soong OS |

---

## 8. Contact

**Josh** — Project Lead for both Soong Path and a2x.

Soong Path is the passion. a2x is the tool that makes it possible. Both matter, but they have different futures — and that's by design.

---

*ColdStart Intelligence Labs — Precision. Clarity. Operator-Grade.*
*A2X Team Briefing — 2026-07-05*
