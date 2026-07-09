# Contributing

We welcome contributions to A2X! Here's how to get started.

## Development Setup

```bash
git clone https://github.com/CanadianCowboy/a2x.git
cd a2x

# Build everything
cargo build

# Run all tests
cargo test --workspace

# Run clippy
cargo clippy --workspace --all-targets

# Format code
cargo fmt --all
```

## Project Structure

```
crates/
  a2x-core/        # Shared types (ConceptVector, WorldGraph, StateField)
  a2x-sigma/       # Σ∞ protocol (tokenizer, parser, binary encoding)
  a2x-omega/       # Ω compiler (7-stage pipeline, optimizer)
  a2x-ccs/         # CCS VM (WorldGraph, StateField, 7 operators)
  a2x-bus/         # Agent communication (discovery, routing, TCP/TLS)
  a2x-agents/      # Agent implementations (ChatAgent, Orchestrator)
  a2x-gateway/     # Entity gateway (HTTP/WS/TCP listeners, dashboard)
  a2x-cli/         # Command-line interface
  a2x-probe/       # Debugging (breakpoints, tracer, inspector)
  a2x-startup/     # Boot/shutdown (config, persistence, resilience)
  a2x-client/      # Rust SDK for external apps
```

## Coding Conventions

- **Rust edition:** 2021
- **Formatting:** `cargo fmt` (default settings)
- **Linting:** `cargo clippy` — must pass with zero warnings
- **Tests:** Every feature should have tests
- **Documentation:** `///` doc comments on all public APIs

## Work Reports & Changelog

When you complete a significant piece of work, write a work report:

1. Copy `work-reports/TEMPLATE.md` to `work-reports/YYYY-MM-DD-description.md`
2. Fill in the sections
3. Run `./scripts/update-changelog.sh --apply` to update `CHANGELOG.md`
4. Commit `CHANGELOG.md` (work reports stay local)

Work reports are gitignored — only the changelog is pushed to remote.

## Pull Request Checklist

- [ ] `cargo build` succeeds
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace --all-targets` has zero warnings
- [ ] `cargo fmt --all` produces no changes
- [ ] New features have tests
- [ ] Public APIs have doc comments
- [ ] CHANGELOG.md updated (if applicable)

## Pre-commit Hooks

```bash
./scripts/setup-hooks.sh
```

Installs git hooks that run fmt, clippy, and tests before each commit.
