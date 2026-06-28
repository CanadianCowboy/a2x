# A2X Ecosystem — Crate Structure, Git Workflow, Testing, CI/CD & Contributions

> **The engineering infrastructure that makes the A2X project buildable, testable, releasable, and extensible by others.**

---

## 1. Overview

This plan covers the non-language aspects of the A2X project: how crates are organized, how we use Git, what our testing/CI pipeline looks like, how we version and release, and how others can contribute.

---

## 2. Crate Structure & Workspace

### Philosophy

Modeled after successful Rust ecosystems (tokio, serde, clap):
- **Workspace root** — virtual manifest organizing all official crates
- **Each crate independently usable** — via git dependencies or local paths
- **Layered dependency** — core crates have zero/minimal deps
- **Feature gates** — optional functionality behind Cargo features
- **Self-hosted ecosystem** — no crates.io, git dependencies only

### Recommended External Crates

| Category | Crate | Why |
|----------|-------|-----|
| Graphs | `petgraph` | Graph data structures + algorithms |
| Tensors | `ndarray` | N-dimensional arrays, NumPy-like |
| Serialization | `serde` + `serde_json` + `bincode` | Data interchange |
| Async | `tokio` | Industry-standard async runtime |
| CLI | `clap` | Derive-based argument parsing |
| Logging | `tracing` | Structured, async-aware diagnostics |
| Errors (lib) | `thiserror` | Derive macro for error enums |
| Errors (app) | `anyhow` | Ergonomic error handling |
| Git | `gix` | Pure-Rust Git implementation |
| ML inference | `candle` (optional) | GPU-accelerated tensor ops |

### Official Crates

| Crate | Purpose | Dependencies |
|-------|---------|--------------|
| `a2x-core` | Primitive types, traits, enums | None |
| `a2x-sigma` | Σ∞ tokenizer, parser, program types | `a2x-core` |
| `a2x-omega` | Ω packets, compiler pipeline | `a2x-core`, `ndarray` (opt) |
| `a2x-bus` | Message bus, routing, transport | `a2x-core`, `a2x-sigma` |
| `a2x-ccs` | CCS VM, WorldGraph, StateField | `a2x-core`, `petgraph`, `ndarray` (opt) |
| `a2x-agents` | Built-in agent implementations | `a2x-core`, `a2x-sigma`, `a2x-bus`, `a2x-ccs` |
| `a2x-gateway` | Entity gateway, protocol listeners | `a2x-bus`, `a2x-sigma` |
| `a2x-client` | Rust client SDK for entities | `a2x-core`, `reqwest` |
| `a2x-cli` | CLI binary | `clap`, `a2x-agents`, `a2x-bus` |
| `a2x-probe` | Debug/probe tools | `a2x-ccs`, `tracing` |

### Feature Gating Example

```toml
# a2x-core/Cargo.toml
[features]
default = ["std"]
std = []
serde = ["dep:serde"]

# a2x-omega/Cargo.toml
[features]
default = []
ndarray = ["dep:ndarray"]
candle = ["dep:candle"]
```

### Third-Party Dependency Pattern

```toml
# third-party/a2x-web-agent/Cargo.toml
[dependencies]
a2x-core = { git = "https://github.com/your-org/a2x", tag = "v0.1.0" }
a2x-bus = { git = "https://github.com/your-org/a2x", tag = "v0.1.0" }
```

---

## 3. Git Workflow

### Branching

| Branch | Purpose |
|--------|---------|
| `main` | Stable, released code |
| `develop` | Integration branch for next release |
| `feature/*` | Individual features |
| `release/v*` | Release preparation |
| `hotfix/*` | Urgent bug fixes |

### Commit Convention (Conventional Commits)

```
feat(core): add ConceptVector operations
fix(sigma): handle malformed boundary tokens
docs(plan): update architecture diagram
test(agents): add orchestrator dispatch tests
perf(omega): optimize tensor slice access
```

### Pre-commit Hooks

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --lib
cargo build --workspace
```

---

## 4. File System Integration

### Config & Data Paths

```
~/.a2x/config.toml
~/.a2x/packets/<date>/<packet-files>
~/.a2x/data/<agent-id>/worldgraph.bin
~/.a2x/data/<agent-id>/memory.bin
~/.a2x/logs/<agent-id>.log
~/.a2x/agents/<agent-id>.toml
~/.a2x/gateway.toml
```

### Logger Format

```
2026-06-28T10:30:00.123Z TRACE a2x::bus: ⟦Σ∞⟧⟬I:⚡✣⩫...⟭
2026-06-28T10:30:00.456Z INFO  a2x::agents::cli: Executing plan: scan ports on sys
2026-06-28T10:30:01.234Z WARN  a2x::agents::cli: Anomaly detected on port 22
```

---

## 5. Testing Strategy

| Level | Tool | What We Test |
|-------|------|-------------|
| **Unit** | `cargo test --lib` | Individual operators, tokenizer, parser |
| **Property** | `proptest` | Tokenizer/parser roundtrip |
| **Integration** | `cargo test --test *` | Multi-agent exchange, bus routing |
| **Fuzz** | `cargo-fuzz` | Parser with malformed input |
| **Benchmark** | `criterion` | Tokenizer throughput, bus latency |
| **Doc tests** | `cargo test --doc` | Examples in documentation |

### Test Structure Example

```
a2x-sigma/
├── src/...
├── tests/
│   ├── parse_valid.rs
│   ├── parse_malformed.rs
│   └── proptest.rs
└── benches/
    └── tokenizer.rs
```

---

## 6. CI/CD Pipeline

### GitHub Actions CI

```yaml
name: CI
on: [push, pull_request]
jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo fmt --all --check
      - run: cargo clippy --workspace --all-targets --all-features

  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: Swatinem/rust-cache@v2
      - run: cargo build --workspace --all-features
      - run: cargo build --workspace --no-default-features

  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: Swatinem/rust-cache@v2
      - run: cargo test --workspace --all-features
      - run: cargo test --workspace --no-default-features
```

### Release Workflow

```yaml
name: Release
on: { push: { tags: ["v*"] } }
jobs:
  release:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo build --workspace --all-features
      - run: cargo test --workspace --all-features
      - uses: softprops/action-gh-release@v1
        with: { generate_release_notes: true }
```

---

## 7. Versioning

**Strategy:** Unified SemVer — all crates share the same version number:
- Tag `v0.2.0` means all crates are at 0.2.0
- Breaking change in any crate bumps the major version for all
- Inter-crate deps use `path = "../other-crate"` for local development

**Release process:**
```bash
# Update version in root Cargo.toml, commit, tag, push
git tag -a v0.2.0 -m "Release v0.2.0"
git push --tags
# CI builds, tests, creates GitHub Release
```

**MSRV:** `rust-version = "1.75.0"` in workspace root.

---

## 8. Contribution Model

### How Others Use A2X

```toml
# third-party/a2x-web-agent/Cargo.toml
[dependencies]
a2x-core = { git = "https://github.com/your-org/a2x", tag = "v0.1.0" }
a2x-bus = { git = "https://github.com/your-org/a2x", tag = "v0.1.0" }
```

Just implement `a2x_core::agent::Agent` and optionally `a2x_bus::transport::Transport`.

### How to Define a Custom Instruction (opcode 0xF)

```rust
use a2x_core::instruction::{CustomInstruction, CustomHandler};

pub struct CryptoHandler;

impl CustomHandler for CryptoHandler {
    fn extension_id(&self) -> [u8; 4] { *b"crpt" }
    fn execute(&self, vm: &mut CcsVm, data: &[u8]) -> Result<(), VmError> {
        // Custom instruction logic
    }
}
```

### Official vs. Third-Party

| Type | Location | How It's Used | Review |
|------|----------|---------------|--------|
| **Official** | `crates/` in this repo | `path = "../other-crate"` | PR review + CI |
| **Third-party** | Separate repos | Git dependency, tag pinning | Self-managed |
| **Curated** | `awesome-a2x` list | Links to third-party repos | PR to list |

### Contributing

1. Fork the repo
2. Create feature branch (`feature/your-feature`)
3. Add changes + tests
4. Run `cargo clippy --workspace -D warnings && cargo test --workspace`
5. Open a PR with clear description
6. CI must pass
7. Maintainer review + merge

---

## 9. Performance Targets (Criterion Benchmarks)

| Benchmark | Target | Crate |
|-----------|--------|-------|
| Σ∞ tokenizer throughput | > 1M packets/sec | `a2x-sigma` |
| Σ∞ parser throughput | > 500K packets/sec | `a2x-sigma` |
| Ω packet encode/decode | > 5M packets/sec | `a2x-omega` |
| Bus message routing | < 100µs latency | `a2x-bus` |
| WorldGraph query | < 1µs per neighbor | `a2x-ccs` |

---

*This sub-plan applies to all phases of the implementation roadmap.*
