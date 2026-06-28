# A2X Documentation Strategy — API Docs, User Guides & Protocol Reference

> **How the A2X project is documented, what tools we use, and what documents we maintain.**

---

## 1. Overview

A2X has multiple audiences that need different documentation:

| Audience | Needs | Format |
|----------|-------|--------|
| **AI assistants (Codebuff)** | Project context, decisions, working style | `README.md` + `.a2x-context.md` |
| **Developers (building A2X)** | Architecture, crate APIs, contribution guide | Rustdoc + `CONTRIBUTING.md` + sub-plans |
| **Integrators (connecting to A2X)** | Gateway endpoints, client SDK usage, entity auth | mdbook: "A2X Integration Guide" |
| **Agent developers (third-party)** | Agent trait, custom instructions, bus protocol | mdbook: "A2X Agent Developer Guide" |
| **Operators (running A2X)** | CLI commands, config file reference, deployment | mdbook: "A2X Operations Guide" |
| **Protocol designers** | Σ∞ ISA reference, Ω spec, CCS architecture | mdbook: "A2X Protocol Reference" |
| **Hobbyists (curious)** | Project overview, quickstart, examples | README + mdbook introduction |

---

## 2. Documentation Tools

| Tool | Purpose | Output |
|------|---------|--------|
| `rustdoc` | API documentation for all crates | HTML (published to GitHub Pages) |
| `mdbook` | Multi-chapter user/developer guides | HTML book (published to GitHub Pages) |
| Markdown | Plans, specs, contribution docs | Plain-text in repo |
| `cargo-doc` | Generate + open rustdoc locally | Local HTML |

---

## 3. Document Inventory

### Repository Root

| File | Purpose | Maintained By |
|------|---------|:-------------:|
| `README.md` | Project overview, quick start, AI context | Core team |
| `PLAN.md` | High-level architecture + all plans index | Core team |
| `CONTRIBUTING.md` | How to contribute | Core team |
| `CODE_OF_CONDUCT.md` | Standards | Core team |
| `LICENSE` | License file | Core team |
| `.github/ISSUE_TEMPLATE/` | Bug reports + feature requests | Core team |

### Sub-Plans (`plans/`)

These are **living design documents** — updated as the project evolves:

| # | File | Purpose |
|:-:|------|---------|
| 00 | `README.md` | Index of all sub-plans |
| 01 | `01-sigma-language.md` | Σ∞ ISA spec |
| 02 | `02-omega-compiler.md` | Ω compilation pipeline |
| 03 | `03-ccs-vm.md` | CCS runtime design |
| 04 | `04-bus.md` | Message bus protocol |
| 05 | `05-agents.md` | Agent types and lifecycle |
| 06 | `06-entity-gateway.md` | Entity integration |
| 07 | `07-probe.md` | Debug protocol |
| 08 | `08-ecosystem.md` | Crate structure + CI/CD |
| 09 | `09-core-types.md` | Core type definitions |
| 10 | `10-concurrency.md` | Concurrency model |
| 11 | `11-startup-shutdown.md` | Boot order + lifecycle |
| 12 | `12-security.md` | Security model |
| 13 | `13-documentation.md` | This document |
| 14 | `14-resilience.md` | Fault tolerance |
| 15 | `15-wasm.md` | WASM support |

### mdbook: "A2X Protocol Reference"

This book is the **canonical spec** — generated from the sub-plans but organized for external readers:

```
src/
├── SUMMARY.md
├── introduction.md
├── 01-overview.md              # Architecture overview
├── 02-sigma-isa.md             # Σ∞ instruction set
├── 03-sigma-operators.md       # Operator tables
├── 04-sigma-programs.md        # Program composition
├── 05-omega-compiler.md        # Compilation
├── 06-omega-packet.md          # Packet format
├── 07-ccs-vm.md                # VM execution
├── 08-ccs-memory.md            # Memory model
├── 09-bus-protocol.md          # Wire format
├── 10-bus-routing.md           # Discovery + routing
├── 11-agent-types.md           # Agent roles
├── 12-agent-lifecycle.md       # State machine
├── 13-gateway.md               # Entity gateway
├── 14-probe-debug.md           # Debug protocol
├── 15-security.md              # Security model
└── appendices/
    ├── A-unicode-table.md       # All Unicode symbols
    ├── B-binary-encoding.md     # Binary wire format
    ├── C-config-reference.md    # Config file reference
    └── D-error-codes.md         # Error type reference
```

### Rustdoc (`cargo doc`)

Each crate has **doc comments** on all public items:

```rust
/// A dense embedding representing a concept, object, event, or abstraction.
///
/// This is the atomic value type in the A2X language. Every concept
/// that an agent reasons about is represented as a ConceptVector.
///
/// # Example
///
/// ```rust
/// use a2x_core::ConceptVector;
///
/// let sys_concept = ConceptVector::from_vec(vec![0.1, 0.2, 0.3]);
/// let other = ConceptVector::zeros(3);
/// let similarity = sys_concept.cosine_similarity(&other);
/// assert!(similarity >= 0.0 && similarity <= 1.0);
/// ```
///
/// # Feature gate
///
/// Serialization requires the `serde` feature.
#[derive(Clone, Debug, PartialEq)]
pub struct ConceptVector { /* ... */ }
```

### Examples (`examples/`)

Working, runnable example programs:

```
examples/
├── hello-world.sigma           # Minimal Σ∞ program
├── sigma-chat.rs               # Two agents chatting in Σ∞
├── multi-agent.rs              # Orchestrator + CLI + CCS agents
├── entity-gateway.rs           # Gateway with HTTP entity
├── probe-agent.rs              # Probe tool inspecting a running agent
└── custom-instruction.rs       # Third-party custom opcode
```

---

## 4. Documentation Generation

### CI/CD Integration

```yaml
# .github/workflows/docs.yml
name: Documentation

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  rustdoc:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo doc --workspace --no-deps
      - uses: actions/upload-pages-artifact@v2
        with:
          path: target/doc

  mdbook:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: peaceiris/actions-mdbook@v1
      - run: mdbook build docs/protocol-reference
      - uses: actions/upload-pages-artifact@v2
        with:
          path: docs/protocol-reference/book

  deploy:
    needs: [rustdoc, mdbook]
    if: github.ref == 'refs/heads/main'
    permissions:
      pages: write
      id-token: write
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    steps:
      - uses: actions/deploy-pages@v2
```

### Published URLs

| Resource | URL |
|----------|-----|
| Rustdoc (all crates) | `https://your-org.github.io/a2x/rustdoc/` |
| Protocol Reference | `https://your-org.github.io/a2x/protocol/` |

---

## 5. Documentation Standards

### What Gets Doc Comments

| Visibility | Required | Optional |
|-----------|:--------:|:--------:|
| `pub` items | ✅ Doc comment with example | — |
| `pub(crate)` items | ✅ Doc comment | — |
| Private items | ❌ | Doc comment for complex logic |
| `pub fn` signatures | ✅ Explain params, return, errors | Example usage |
| `pub trait` | ✅ Explain what it does + implementors | Example impl |
| `pub enum` | ✅ Explain each variant | — |
| `pub struct` | ✅ Explain what it represents | Field docs |
| `pub mod` | ✅ A few sentences about the module | — |

### Commit Message Documentation

Substantial changes to architecture should update the relevant sub-plan:
```
feat(ccs): implement parallel swarm with fork-join

Also updated plans/03-ccs-vm.md with merge semantics.
```

---

## 6. README.md (AI Context File)

The `README.md` serves dual purpose: project intro for humans AND working context for AI assistants.

**Kept current with:**
- Key decisions (name, crate prefix, ecosystem model)
- Current phase (what's being worked on now)
- Working style preferences (interactive, plan-first)
- Quick reference table (crates, layers, architecture)
- File tree

---

*This sub-plan applies to all phases; documentation is continuous.*
