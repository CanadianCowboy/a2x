# Agent Model

A2X agents are autonomous cognitive entities that communicate through a shared
bus. Each agent has a type, capabilities, and a lifecycle.

## Agent Types

| Type | Description |
|------|-------------|
| `Orchestrator` | Top-level coordinator — dispatches programs, manages sub-agents |
| `CLI` | Command-line interface agent — exposes CLI commands |
| `CCS` | Cognitive agent — runs the CCS VM tick loop with introspection |
| `LLM` | Large language model agent — ChatAgent with Ollama/OpenAI backend |
| `Gateway` | Entity gateway — manages external connections and auth |
| `Custom` | User-defined agent types |

## Agent Identity

Each agent has a unique `AgentId` composed of:

- **Type** — `Orchestrator`, `CLI`, `CCS`, `LLM`, `Gateway`, or `Custom`
- **Name** — a human-readable identifier (e.g., `"chat-agent-1"`)
- **UUID** — a unique identifier for deduplication

```rust
use a2x_core::agent_id::{AgentId, AgentType};

let id = AgentId::new(AgentType::CCS, "my-ccs-agent");
```

## Agent Capabilities

Agents advertise what they can do via `Capability` flags:

| Capability | Description |
|------------|-------------|
| `Parse` | Can parse Σ∞ programs |
| `Compile` | Can compile to Ω tensors |
| `Execute` | Can run programs on the CCS VM |
| `Probe` | Can debug/trace execution |
| `Chat` | Can converse with users |
| `Orchestrate` | Can coordinate sub-agents |
| `Discover` | Can find other agents on the bus |

## Agent Lifecycle

```
  Created → Registered → Online → (Working) → Offline → Unregistered
                ↑                      ↓
                └──────────────────────┘
                    (re-registration)
```

States:
1. **Created** — agent struct exists, not yet on the bus
2. **Registered** — agent published to the bus with capabilities
3. **Online** — agent is active and responding to messages
4. **Offline** — agent has stopped (graceful or crash)
5. **Unregistered** — agent removed from the bus

## Built-in Agents

### ChatAgent

The ChatAgent is an LLM-powered conversational agent:

- **Backends:** Ollama (local) or OpenAI (cloud)
- **Tools:** 14 built-in tools (Sigma execution, CCS VM ops, bus discovery, etc.)
- **Memory:** Sliding window history with context injection
- **Persistence:** Auto-save/load conversations to `~/.a2x/conversations/`

### Orchestrator

The Orchestrator coordinates multi-agent workflows:

- Dispatches programs to CCS agents
- Merges results from parallel execution
- Manages agent lifecycle (spawn, stop, restart)

### CCS Agent

The CCS Agent runs a cognitive tick loop:

1. Poll the bus for incoming programs
2. Execute each program on the CCS VM
3. Publish results back to the bus
4. Run introspection (probe) if configured
