# Phase 5 — Probe & Interpretability (CCS Debug Interface)

**Date:** 2026-06-28
**Scope:** `a2x-ccs` probe channel integration + `a2x-probe` crate

---

## What Was Built

### 5.1 Extended Probe Types (`a2x-ccs/src/probe.rs`)

- **`BreakpointType`**: `Instruction(usize)`, `Opcode(Opcode)`, `AfterSteps(u64)`
- **`TracerMode`**: `Off` / `Light` / `Full` / `Verbose` (with `Display` impl)
- **`ProbeEvent`**: `BreakpointHit` / `Stepped` / `Halted` / `Faulted` (with `Display` impl)
- New `ProbeQuery` variants: `SetTracerMode`, `ListBreakpoints`, `ClearAllBreakpoints`, `ListRegions`, `GraphSummary`

### 5.2 Probe Channel in CcsVm (`a2x-ccs/src/vm.rs`)

**New fields on `CcsVm`:**
- `probe_rx: Option<Receiver<ProbeQuery>>` — checked between each instruction
- `probe_event_tx: Option<Sender<ProbeEvent>>` — fires events to the probe tool
- `breakpoints: HashMap<usize, BreakpointType>` — IP → breakpoint type
- `stepping: bool` — single-step mode (re-pause after one instruction)
- `tracer_mode: TracerMode` — controls logging verbosity
- `watchdog_steps: Option<u64>` — optional step limit

**Key methods:**
- `attach_probe()` → `(Sender<ProbeQuery>, Receiver<ProbeEvent>)`
- `handle_probe_queries()` — borrows `probe_rx` only for `try_recv()`, drops before mutation
- `process_probe_query()` — handles individual queries (avoids borrow conflicts)
- `run_probed()` — main loop: probe checks → breakpoint detection → step → stepping re-pause
- `probe_snapshot()`, `probe_node()`, `probe_region()`, `probe_trace_tail()` — snapshot builders

### 5.3 a2x-probe Crate (`crates/a2x-probe/`)

- **`ProbeTool`**: Connects to CcsVm via mpsc channels, sends queries, receives events
- **`ProbeError`**: `ChannelClosed` / `NotConnected`
- **Visualization helpers**: `format_snapshot()`, `world_graph_to_dot()`, `state_field_summary()`
- 7 unit tests covering round-trip, formatting, dot generation, and error display

### 5.4 Borrow Safety Design

The critical design decision: `handle_probe_queries()` borrows `self.probe_rx` only for
the duration of `try_recv()`, then drops the borrow before calling `process_probe_query()`
which mutates `self.paused`, `self.stepping`, `self.breakpoints`, etc. This avoids the
E0502 borrow conflict that occurred when `probe_rx` was held across the loop body.

---

## Validation

- ✅ 133 a2x-ccs tests pass (including 6 phase2_smoke integration tests)
- ✅ 13 a2x-probe tests pass (7 new + 6 existing)
- ✅ `cargo clippy -p a2x-ccs -p a2x-probe --all-targets -- -D warnings` clean
- ✅ `cargo fmt` clean
