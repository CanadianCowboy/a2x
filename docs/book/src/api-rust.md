# Rust Crate Docs

A2X is organized as a Cargo workspace with 12 crates. Each crate has its own
API surface documented here.

## Core Crate: `a2x-core`

```rust
// Fundamental types used by all crates
use a2x_core::{
    agent::{Agent, AgentState},
    agent_id::{AgentId, AgentType},
    capability::Capability,
    concept::ConceptVector,
    error::A2xError,
    graph::WorldGraph,
    memory::MemoryTrace,
    node::NodeId,
    relation::{RelationEdge, RelationType},
    state::StateField,
};
```

## Sigma Crate: `a2x-sigma`

```rust
// Σ∞ protocol parsing and serialization
use a2x_sigma::{
    parse_program,
    tokenizer::{Tokenizer, Token},
    parser::Parser,
    SigmaProgram, SigmaPacket,
    intent::IntentOp,
    context::ContextOp,
    plan::PlanOp,
    data::DataOp,
};
```

## Omega Crate: `a2x-omega`

```rust
// Ω compilation pipeline
use a2x_omega::{
    CompileToOmega,
    OptimizationLevel,
    OmegaProgram,
    OmegaPacket,
    passes::{constant_folding, dead_code, fusion, layout},
};
```

## CCS Crate: `a2x-ccs`

```rust
// Cognitive Control Substrate VM
use a2x_ccs::{
    CcsVm, AsyncCcsVm,
    state::StateField,
    safety::SafetyLevel,
    operators::{bind, differentiate, ground, evolve, reflect, plan, actuate},
    probe::ProbeApi,
};
```

## Bus Crate: `a2x-bus`

```rust
// Agent communication bus
use a2x_bus::{
    Bus, BusBridge,
    AgentFilter,
    discovery::AgentDiscovery,
    routing::MessageRouter,
    transport::{Transport, TcpTransport, TlsTransport},
};
```

## Agents Crate: `a2x-agents`

```rust
// Agent implementations
use a2x_agents::{
    ChatAgent, ChatConfig,
    Orchestrator,
    CcsAgent,
    llm_backend::{OllamaBackend, OpenAiBackend},
};
```

## Gateway Crate: `a2x-gateway`

```rust
// Entity gateway
use a2x_gateway::{
    Gateway, GatewayConfig,
    listeners::{
        HttpListener, WsListener, TcpListener, StdioListener,
        ProtocolListener,
    },
    auth::AuthMethod,
    dashboard::Dashboard,
};
```

## Client Crate: `a2x-client`

```rust
// Rust SDK for external applications
use a2x_client::A2xClient;

let client = A2xClient::connect("http://localhost:8778")?;
let result = client.execute("⟦Σ∞⟧⟬I:✦ ∷ C:⟨test⟩ ∷ P:⥂ ∷ D:⌬⟭").await?;
```

## Probe Crate: `a2x-probe`

```rust
// Debugging and introspection
use a2x_probe::{
    Probe, Tracer, Inspector,
    Breakpoint, BreakpointKind,
    TraceConfig, Verbosity,
};
```

## Startup Crate: `a2x-startup`

```rust
// Production boot/shutdown management
use a2x_startup::{
    boot, BootConfig,
    persistence::PersistState,
    resilience::ResilienceConfig,
    shutdown::graceful_shutdown,
};
```

For comprehensive API documentation, run:

```bash
cargo doc --workspace --open
```
