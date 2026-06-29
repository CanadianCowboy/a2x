# Phase 5 — Probe & Interpretability (CCS Debug Interface)

> **Date:** 2026-06-28  
> **Tag:** v0.5.0  
> **Commits:** 58045d6 (initial), gap-fill commit pending  
> **Scope:** plans/07-probe.md, PLAN.md §18

---

## Summary

Phase 5 implements the debug/probe interface for inspecting running CCS VMs — breakpoints, single-stepping, state inspection, tracer modes, CLI commands, and visualization helpers.

---

## Deliverables

### 5.1 Probe Protocol Types (a2x-ccs/probe.rs)

| Type | Description |
|------|-------------|
| `ProbeQuery` | 17 variants: Snapshot, GetIp, GetNode, GetNodeByLabel, GetRegion, GetPc, GetTraceTail, SetBreakpoint, ClearBreakpoint, Step, Continue, SetTracerMode, ListBreakpoints, ClearAllBreakpoints, ListRegions, GraphSummary |
| `BreakpointType` | 6 variants: Instruction(usize), Opcode(Opcode), AfterSteps(u64), NodeAccess{label, access_type}, RegionAccess{region, access_type}, Conditional{condition} |
| `AccessType` | Read, Write, Both |
| `Condition` | AtInstruction(usize), AfterSteps(u64), Custom(String) |
| `TracerMode` | Off, Light, Full, Verbose (default: Verbose) |
| `ProbeEvent` | BreakpointHit, Stepped, Halted, Faulted — all with Display |
| `ProbeSnapshot` | 12 variants: VmState, Node, Region, QueryResult, TraceSegment, BreakpointSet, BreakpointCleared, Stepped, Continued, BreakpointList, RegionList, GraphSummary, TraceLog |
| `TraceLogEntry` | Per-instruction trace data: ip, opcode, steps, state_summary, trace_len |

### 5.2 CcsVm Probe Channel (a2x-ccs/vm.rs)

| Method | Purpose |
|--------|---------|
| `attach_probe()` | Returns mpsc channel pair (query_tx, event_rx) |
| `handle_probe_queries()` | Drains probe_rx non-blocking, calls process_probe_query per query |
| `process_probe_query()` | Handles all 17 ProbeQuery variants |
| `run_probed()` | Execution loop with breakpoint detection (instruction + advanced), stepping re-pause, watchdog, tracer logging |
| `eval_condition()` | Evaluates Condition against VM state |
| `region_names()` | Returns StateField region list |
| `graph_summary()` | Returns (node_count, edge_count) |
| `tracer_mode()` | Accessor |
| `tracer_log()` | Accessor |
| `tracer_log_len()` | Accessor |

### 5.3 a2x-probe Crate

| Module | Contents |
|--------|----------|
| `lib.rs` | ProbeTool, ProbeError, ProbeExt trait, format_snapshot(), world_graph_to_dot(), state_field_summary(), heatmap_ascii() |
| `tracer.rs` | Tracer: format_entry, format_entries, format_tail, timeline (ASCII IP bar), state_heatmap (ASCII heatmap) |
| `inspector.rs` | execute_command() CLI dispatcher: status, graph, regions, break, clear, continue, step, trace, watch, tracer, heatmap, timeline, help, quit |

### 5.4 Visualization Helpers

| Helper | Description |
|--------|-------------|
| `world_graph_to_dot()` | Graphviz DOT output with label escaping |
| `state_field_summary()` | ASCII table of StateField regions |
| `heatmap_ascii()` | ASCII heatmap mapping [-1,1] to ` .:-=+*#%@` |
| `Tracer::timeline()` | ASCII bar showing IP execution positions |
| `Tracer::state_heatmap()` | Multi-entry heatmap across trace entries |

---

## Test Coverage

| Crate | Tests | Status |
|-------|-------|--------|
| a2x-ccs | 144 + 6 + 36 | ✅ All pass |
| a2x-probe | 36 + 7 (tracer) + 11 (inspector) | ✅ All pass |

**Clippy:** Clean (0 warnings)  
**Fmt:** Clean

---

## What Was Implemented vs. plans/07-probe.md

| Section | Deliverable | Status |
|---------|-------------|--------|
| §1 | a2x-ccs + a2x-probe deps | ✅ |
| §2 | ProbeQuery / ProbeSnapshot | ✅ (17 + 12 variants) |
| §3 | Probe channel connection | ✅ (mpsc) |
| §4 | Channel separation | ✅ (between-instruction check) |
| §5 | BreakpointType (all variants) | ✅ (6 types) |
| §5 | AccessType, Condition | ✅ |
| §5 | Breakpoint lifecycle | ✅ (HashMap, O(1) lookup) |
| §6 | TracerMode enum | ✅ (4 modes) |
| §6 | TracerMode wired into execution loop | ✅ (per-instruction logging) |
| §7 | CLI probe commands | ✅ (13 commands) |
| §8 | ProbeExt trait | ✅ (defined) |
| §9 | WorldGraph graphviz | ✅ (dot output) |
| §9 | StateField heatmap | ✅ (ASCII) |
| §9 | Instruction tracer | ✅ (timeline + format) |
| §9 | MemoryTrace timeline | ✅ (via tracer) |

### Still deferred (future phases):

- `Conditional(Box<dyn Fn>)` — replaced with string-keyed `Custom(String)` for now
- Web dashboard (leptos/dioxus) — Phase 6+
- `ProbeExt` implementation on concrete types — needs bus integration
- Remote probe over network transport — Phase 6+
- `a2x-bus` / `tracing` deps in Cargo.toml — not yet needed

---

## Files Changed

| File | Changes |
|------|---------|
| `crates/a2x-ccs/src/probe.rs` | Extended with AccessType, Condition, TraceLogEntry, advanced breakpoint types, new ProbeSnapshot variants |
| `crates/a2x-ccs/src/vm.rs` | tracer_log field, tracer_mode/log/log_len accessors, TracerMode wiring in run_probed(), advanced breakpoint checking, eval_condition, region_names, graph_summary |
| `crates/a2x-ccs/src/lib.rs` | Updated re-exports |
| `crates/a2x-probe/src/lib.rs` | New modules, ProbeExt trait, heatmap_ascii, label escaping, new snapshot formatting |
| `crates/a2x-probe/src/tracer.rs` | New: Tracer with formatting, timeline, heatmap |
| `crates/a2x-probe/src/inspector.rs` | New: CLI command dispatcher |

---

## Next Steps

- Phase 6: Entity Integration (a2x-gateway, protocol listeners)
- Implement `ProbeExt` on concrete agent types
- Add remote probe over bus transport
- Wire `ProbeQuery` variants for advanced breakpoint types
