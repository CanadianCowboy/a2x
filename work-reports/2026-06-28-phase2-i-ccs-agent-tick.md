# Phase 2.I — CcsAgent::tick (3-instruction cognitive-loop driver)

**Date:** 2026-06-28
**Branch:** `feature/phase2-ccs-operator-body`
**Status:** ✅ green — 130 a2x-ccs + 38 a2x-agents (10 new on agents). All clippy runs clean.

## What landed

PLAN.md §18 item "CCS agent that maintains a world-model" now has a real wire-in to the CCS VM. The persistent VM is the agent's world, and `tick()` drives one cognitive cycle.

**`CcsAgent::tick() -> Result<TickResult, AgentError>`** synthesizes a 3-instruction Σ∞ program (`[Delay/Contradiction/Parallel → Evolve/Reflect/Plan]`), loads+vm.run() under the existing VM mutex, snapshots observations, and returns a cloned `TickResult`. Locking is tight — mutex released before any caller-side work.

**`CcsAgent::run_program(SigmaProgram) -> Result<TickResult, AgentError>`** allows callers (e.g. CLI agent) to run their own parsed Σ∞ programs and still get the same observability surface.

**`CcsAgent::vm_snapshot() -> Option<VmSnapshot>`** is a read-only borrow — `VmSnapshot { ip, steps_executed, world_graph_size, memory_trace_length, last_reflect_set, plan_actions, uptime }` — for state inspection without consuming programs.

**`TickResult`** struct derives Clone+Debug+PartialEq — Clone because VM mutex must release before user code (lock held only during load+run).

New tests (10):
- `test_tick_returns_three_steps_executed`
- `test_tick_appends_trace_entries`
- `test_tick_sets_last_reflect`
- `test_tick_records_plan_actions`
- `test_tick_grows_world_graph`
- `test_two_ticks_increments_steps_by_three_each`
- `test_tick_when_not_running_still_works`
- `test_run_program_with_halted_program_returns_tick`
- `test_run_program_with_single_evolve_advances_state`

## Files touched

| Path | Δ | Purpose |
|---|---|---|
| `crates/a2x-agents/src/ccs_agent.rs` | +250 (rewrite) | `tick`, `run_program`, `vm_snapshot`, `TickResult`, `VmSnapshot`, 10 tests |

## Design notes

- **Mutex discipline:** VM mutex held only for load+run; observations cloned before release. Avoids holding a potentially-poisoned lock across user code or async boundaries.
- **Program = canonical 3-instruction:** Delay + Contradiction + Parallel → OpCodes Evolve / Reflect / Plan. The VM naturally halts at end-of-packet-stream, no explicit HALT needed.
- **Observable invariants** the test suite guards: steps_executed == 3, memory_trace_length == 3, last_reflect_set == true, plan_actions non-empty, world_graph grows by ≥ 2 per tick (self-model + plan nodes).
- **Yield semantics:** `tick()` treats `VmStatus::Yield` as soft-pause (continues to snapshot) — documented in code; future proofing if the VM ever yields mid-program.
