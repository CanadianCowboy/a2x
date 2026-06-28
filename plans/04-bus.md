# A2X Bus — Message Routing & Transport Protocol

> **The message-oriented middleware that routes programs between agents and entities.**

---

## 1. Overview

The bus is the **communication backbone** of the A2X ecosystem. It routes Σ∞ and Ω programs between agents (and entities via the gateway).

- **Crate:** `a2x-bus`
- **Depends on:** `a2x-core`, `a2x-sigma`
- **Key files:** `bus.rs`, `transport.rs`, `routing.rs`, `discovery.rs`, `wire.rs`

---

## 2. Architecture

```
┌──────────┐     ┌──────────────────────────┐     ┌──────────┐
│ Agent A  │────▶│       A2X Bus            │────▶│ Agent B  │
│(Orch.)   │     │                          │     │(CLI)     │
└──────────┘     │  ┌────────┐ ┌─────────┐  │     └──────────┘
                 │  │ Router │ │ Discovery│  │
┌──────────┐     │  └────────┘ └─────────┘  │     ┌──────────┐
│ Gateway  │────▶│                          │────▶│ Agent C  │
│(Entities)│     └──────────────────────────┘     │(CCS)     │
└──────────┘                                      └──────────┘
```

---

## 3. Transport Layer

```rust
#[async_trait]
pub trait Transport: Send + Sync {
    async fn bind(&mut self, addr: SocketAddr) -> Result<(), TransportError>;
    async fn connect(&self, addr: SocketAddr) -> Result<Box<dyn Connection>, TransportError>;
    async fn accept(&self) -> Result<Box<dyn Connection>, TransportError>;
}

#[async_trait]
pub trait Connection: Send + Sync {
    async fn send(&self, program: &[u8]) -> Result<(), TransportError>;
    async fn recv(&self) -> Result<Vec<u8>, TransportError>;
    async fn close(&self) -> Result<(), TransportError>;
}
```

### Built-in Transports

| Transport | Use Case |
|-----------|----------|
| **In-memory** | Local agents, same process (channels) |
| **TCP** | Remote agents, same machine or LAN |
| **Unix sockets** | Local agents, different processes |
| **WebSocket** | Browser-based agents, web dashboards |
| **gRPC** | Cross-DC, cloud deployments |

---

## 4. Wire Format

Every message has a uniform header:

```rust
#[derive(Serialize, Deserialize)]
pub struct WireMessage {
    pub version: u8,
    pub msg_type: MessageType,
    pub sender: AgentId,
    pub recipient: Option<AgentId>,
    pub correlation_id: Uuid,
    pub payload: MessagePayload,
    pub timestamp: u128,
}

pub enum MessageType {
    SigmaProgram,                    // Σ∞ program to execute
    OmegaProgram,                    // Ω program to execute
    Announce,                        // Agent discovery / announcement
    ProgramRequest(ProgramId),       // Request cached program by ID
    ProgramResponse(ProgramId, SigmaProgram), // Cached program response
    Error(WireError),                // Error response
    Heartbeat,                       // Keepalive
    ProbeRequest(ProbeQuery),        // Debug probe request
    ProbeResponse(ProbeSnapshot),    // Debug probe response
}
```

Serialization: bincode (compact binary) for everything, JSON for debug/config.

---

## 5. Agent Discovery

Three methods (tried in order):

1. **Static registration** — config file lists known agent addresses
2. **Broadcast announce** — on startup, agents broadcast `Announce` on the bus
3. **Directory service** — central registry (optional, for large deployments)

```rust
#[async_trait]
pub trait Discovery: Send + Sync {
    async fn register(&self, agent: AgentInfo) -> Result<(), DiscoveryError>;
    async fn discover(&self, filter: AgentFilter) -> Result<Vec<AgentInfo>, DiscoveryError>;
    async fn watch(&self) -> BoxStream<DiscoveryEvent>;
}

pub struct AgentInfo {
    pub id: AgentId,
    pub agent_type: AgentType,
    pub addr: SocketAddr,
    pub capabilities: Vec<String>,
    pub version: String,
    pub load: Option<f32>,
}
```

---

## 6. Routing

The router matches programs to agents based on:

1. **Capability matching** — does the agent have the required capabilities?
2. **Load balancing** — among matching agents, pick the least loaded
3. **Affinity** — prefer agent with relevant cached state (WorldGraph locality)

```rust
pub struct Router {
    discovery: Box<dyn Discovery>,
    strategy: RoutingStrategy,
}

pub enum RoutingStrategy {
    FirstMatch,
    LeastLoaded,
    RoundRobin,
    Random,
    ByLabel(String),
}
```

---

## 7. Connection Lifecycle

```
1. Agent A starts, creates bus with in-memory transport
2. Agent A announces: "I am orchestrator-1, capabilities: [plan, dispatch]"
3. Agent B starts (CLI agent), announces: "I am cli-1, capabilities: [exec, fs, net]"
4. Router registers both agents
5. Orchestrator A sends program to bus, addressed to "capability: exec"
6. Router matches to cli-1, delivers program
7. B's CCS VM executes the program
8. B sends result program back to A
9. If B disconnects, router marks it offline, routes to cli-2 if available
```

---

*This sub-plan maps to phases 0–1 of the implementation roadmap.*
