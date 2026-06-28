# A2X Agents — Types, Lifecycle, Safety & Error Model

> **The execution contexts that run A2X programs. Agents are runtimes, not just message handlers.**

---

## 1. Overview

Agents are **execution contexts** for A2X programs. Each agent has a CCS runtime that executes Σ∞/Ω programs and maintains its own WorldGraph + StateField.

- **Crate:** `a2x-agents`
- **Depends on:** `a2x-core`, `a2x-sigma`, `a2x-bus`, `a2x-ccs`
- **Key files:** `orchestrator.rs`, `cli_agent.rs`, `llm_agent.rs`, `ccs_agent.rs`, `lifecycle.rs`

---

## 2. Agent Trait

```rust
#[async_trait]
pub trait Agent: Send + Sync {
    fn id(&self) -> AgentId;
    fn agent_type(&self) -> AgentType;

    /// Execute a Σ∞ program on this agent's CCS runtime.
    async fn execute(&self, program: SigmaProgram) -> Result<SigmaProgram, AgentError>;

    /// Execute a compiled Ω program directly (fast path).
    async fn execute_omega(&self, program: OmegaProgram) -> Result<OmegaProgram, AgentError>;

    /// Current internal state (for probing / debug).
    fn state_summary(&self) -> Option<StateSnapshot>;
}
```

---

## 3. Built-in Agent Types

| Agent Type | Native Form | Role |
|------------|:-----------:|------|
| **Orchestrator** | Σ∞ + Ω | Writes A2X programs, dispatches to other agents for execution |
| **LLM Agent** | Σ∞ (source) + Ω (compiled) | Generates Σ∞ programs from natural language intent; decompiles Ω for inspection |
| **CLI Agent** | Σ∞ (instructions) | Executes Σ∞ programs that interact with the host system (files, processes, network) |
| **CCS Agent** | Ω (native) + Σ∞ (trace) | Maintains a persistent WorldGraph; executes long-running cognitive programs |
| **Ω Agent** | Ω only | Pure latent execution — max speed, zero inspectability |

### Orchestrator

Top-level coordinator. Receives high-level goals, decomposes them into Σ∞ programs, dispatches to other agents, collects results.

### CLI Agent

Executes programs that interact with the host OS:
- Filesystem operations (read, write, list)
- Process execution (run commands, capture output)
- Network operations (connect, scan, fetch)
- System information (CPU, memory, processes)

### LLM Agent

Bridges between natural language and A2X:
- Converts human requests into Σ∞ programs
- Converts Σ∞ results back into natural language explanations
- Decompiles Ω programs to Σ∞ for human inspection

### CCS Agent

Long-running cognitive agent that maintains a persistent WorldGraph:
- Never stops, continuously executes Evolve/Reflect cycles
- Builds up a rich world-model over time
- Responds to queries about its world-model

---

## 4. Agent Lifecycle

### State Machine

```
┌──────────┐    ┌──────────┐    ┌──────────┐
│  Idle    │───▶│  Running │───▶│  Idle    │
└──────────┘    └──────────┘    └──────────┘
     │                │               │
     ▼                ▼               ▼
┌──────────┐    ┌──────────┐    ┌──────────┐
│  Error   │◀───│  Error   │    │  Halted  │
└──────────┘    └──────────┘    └──────────┘
     │
     ▼
┌──────────┐
│  Dead    │
└──────────┘
```

```rust
pub enum AgentState {
    Idle,
    Running { program_id: ProgramId, started_at: Instant, vm: Box<CcsVm> },
    Error { error: AgentError, retry_count: u32 },
    Halted,
    Dead,
}
```

### Configuration

```toml
# ~/.a2x/agents/cli-1.toml
[agent]
id = "cli-1"
type = "cli"
label = "primary execution agent"

[agent.capabilities]
exec = true
fs = true
network = ["tcp", "dns"]

[safety]
level = "bounded"
max_instructions = 10000
max_memory_mb = 256
allowed_commands = ["ls", "ps", "netstat", "cat", "grep", "find"]
forbidden_patterns = ["rm", "sudo", "chmod", "dd", "> /dev/*"]

[bus]
transport = "tcp"
listen = "127.0.0.1:0"
bootstrap = ["127.0.0.1:8777"]

[storage]
worldgraph = "~/.a2x/data/cli-1/worldgraph.bin"
memory = "~/.a2x/data/cli-1/memory.bin"

[logging]
level = "info"
format = "json"
file = "~/.a2x/logs/cli-1.log"
```

### Monitoring & Heartbeats

- Heartbeat every 5 seconds (configurable)
- If no heartbeat for 3× interval, agent presumed dead
- Router marks dead agents offline, redistributes workload
- State saved to disk for crash recovery

---

## 5. Safety Model

### Safety Levels

```rust
pub enum SafetyLevel {
    /// No restrictions. Dev/debug only.
    Unrestricted,
    /// Bounded execution. Limits on loops, memory, side effects.
    Bounded {
        max_instructions: u64,
        max_memory_bytes: u64,
        max_side_effects: u32,
        allowed_syscalls: Vec<String>,
    },
    /// Sandboxed. All side effects filtered through allowlist.
    Sandboxed {
        allowed_commands: Vec<GlobPattern>,
        allowed_network: Vec<String>,
        allowed_files: Vec<PathGlob>,
    },
    /// Full isolation. No side effects. Read-only world-model.
    Isolated,
}
```

### ISA-Level Safety

Safety is baked into instruction encoding:
- **Flags in every instruction header** — carries safety classification
- **Capability bits** — specifies required capabilities (network, filesystem, etc.)
- **Bounds on immediate values** — D field constraints in the type system

```rust
pub struct SafetyClassification {
    pub requires_exec: bool,
    pub requires_fs_read: bool,
    pub requires_fs_write: bool,
    pub requires_network: bool,
    pub max_allocation: Option<u64>,
    pub max_steps: Option<u64>,
}
```

### CLI Agent Sandboxing

```rust
pub struct CliAgent {
    allowed_commands: Vec<GlobPattern>,
    sandbox: SandboxMode,
    max_execution_time: Duration,
    max_retries: u32,
}

pub enum SandboxMode {
    None,
    CommandFilter,     // Filter against allowlist
    Container,         // Docker container (future)
    Vm,                // Micro-VM (future)
}
```

---

## 6. Error Model

### Error Types

| Error | Source | Description |
|-------|--------|-------------|
| `LexError::UnknownCharacter` | Tokenizer | Unrecognized character |
| `ParseError::UnexpectedToken` | Parser | Token doesn't fit format |
| `ParseError::MissingField` | Parser | Required field is empty |
| `SemanticError::UndefinedLabel` | Analyzer | Jump target doesn't exist |
| `SemanticError::TypeMismatch` | Analyzer | Data type mismatch |
| `CompileError::UnsupportedOpcode` | Compiler | Opcode has no Ω encoding |
| `VmError::OutOfMemory` | Runtime | WorldGraph allocation limit |
| `VmError::SafetyViolation` | Runtime | Instruction violates safety |
| `VmError::InvalidAddress` | Runtime | Non-existent memory reference |
| `VmError::ParallelMergeConflict` | Runtime | Fork results conflict |
| `VmError::MaxStepsExceeded` | Runtime | Instruction limit exceeded |
| `AgentError::ProgramCrash(VmError)` | Agent | CCS VM crashed |
| `AgentError::Timeout` | Agent | Program exceeded time limit |
| `TransportError::ConnectionLost` | Bus | Remote agent disconnected |

### Error Recovery

```
Error occurs
    │
    ├── Can recover? ──Yes──→ Retry / Skip / Continue
    │
    └── No
        │
        ├── Has parent? ──Yes──→ Escalate to caller (⤊)
        │
        └── No ──→ Crash: emit error program, halt VM
```

### Error Programs

When a program crashes, the VM produces an error program:
```
⟦Σ∞⟧⟬I:⚠⟁ ∷ C:⟤⟨crash⟩ ∷ P:✕ ∷ D:⌴⟨VmError::OutOfMemory⟩⟭
```

The error program can be returned to the caller, logged, or handled by a supervisor agent.

---

*This sub-plan maps to phases 0–2 of the implementation roadmap.*
