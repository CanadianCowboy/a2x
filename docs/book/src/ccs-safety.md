# Safety & Sandboxing

The CCS VM enforces resource limits and safety constraints during execution.

## Safety Levels

| Level | Enforcement |
|-------|-------------|
| `Production` | Checks `nodes_allocated * 4 KiB` against `max_memory_bytes` |
| `Bounded { max_memory_bytes, max_ip }` | Enforces both memory and IP bounds |
| `Unsafe` | No enforcement (development only) |

## Memory Safety

The VM tracks memory usage:

```rust
use a2x_ccs::safety::SafetyLevel;

let safety = SafetyLevel::Bounded {
    max_memory_bytes: 1024 * 1024,  // 1 MiB
    max_ip: 10_000,                  // 10K instructions
};

let mut vm = CcsVm::with_safety(safety);
```

When limits are exceeded:
- **Memory exhaustion** → `Error::OutOfMemory` — execution halts
- **IP overflow** → `Error::InfiniteLoop` — execution halts
- **Stack overflow** → `Error::StackOverflow` (recursive calls)

## Sandboxed Execution

All VM operations are sandboxed:

- No file system access
- No network access (except through the bus)
- No system calls
- Deterministic execution given the same input

## Fuzzing

The CCS VM has been tested against edge cases:

```rust
// Property: VM should never crash
proptest! {
    fn vm_never_panics(program in arbitrary_program()) {
        let mut vm = CcsVm::new(SafetyLevel::Bounded {
            max_memory_bytes: 1024,
            max_ip: 100,
        }).unwrap();
        // Should always return a Result, never panic
        let _ = vm.load(&program);
        let _ = vm.run();
    }
}
```

## Production Recommendations

For production deployments:

- Use `SafetyLevel::Production` with appropriate `max_memory_bytes`
- Set `max_ip` based on expected program complexity
- Monitor node allocation rate via the probe API
- Review MemoryTrace for anomalous execution patterns
