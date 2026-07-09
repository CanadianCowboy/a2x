# Probe & Debugging

The Probe system provides debugging and introspection for the CCS VM.

## Features

- **Breakpoints** — instruction, opcode, and conditional breakpoints
- **Tracer** — step-by-step execution trace with configurable verbosity
- **Inspector** — live WorldGraph and StateField inspection
- **Timeline** — MemoryTrace viewer
- **WebSocket** — remote debugging via the gateway

## Setting Breakpoints

```rust
use a2x_probe::{Probe, Breakpoint, BreakpointKind};

let mut probe = Probe::new();

// Break at instruction 5
probe.set_breakpoint(Breakpoint::instruction(5));

// Break on any BIND operation
probe.set_breakpoint(Breakpoint::opcode(Opcode::Bind));

// Conditional breakpoint
probe.set_breakpoint(Breakpoint::conditional(10, |state| {
    state.belief_activation() > 0.8
}));
```

## Tracing

```rust
use a2x_probe::Tracer;

let tracer = Tracer::new(TraceConfig {
    verbosity: Verbosity::Verbose,
    capture_state: true,
});

// Attach to a VM
let mut vm = CcsVm::with_probe(probe);
vm.load(&program)?;
vm.run()?;

// Read the trace
for entry in tracer.entries() {
    println!("ip={} op={:?} state_change={:?}",
        entry.ip, entry.opcode, entry.state_delta);
}
```

## Inspector

```rust
use a2x_probe::Inspector;

let inspector = Inspector::attach(&vm);

// Traverse the WorldGraph
let neighbors = inspector.neighbors_of(node_id)?;

// Read state regions
let belief = inspector.read_region("belief")?;

// Get memory stats
let stats = inspector.memory_stats()?;
```

## Remote Debugging

The probe can be accessed remotely through the gateway WebSocket:

```json
// Request
{
  "action": "set_breakpoint",
  "kind": "instruction",
  "address": 5
}

// Response
{
  "ok": true,
  "breakpoint_id": 3
}
```
