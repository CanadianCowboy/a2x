# The Bus

The Bus is A2X's inter-agent communication layer — a publish-subscribe transport
that enables agent discovery, message routing, and identity verification.

## Architecture

```
Agent A ←→ Bus ←→ Agent B
              ↕
          Transport
        (TCP / TLS / In-Process)
```

## Features

- **Discovery** — agents register types and capabilities
- **Routing** — messages delivered by agent ID, type, or capability
- **Filtering** — subscribe to specific message types
- **Identity** — Ed25519 signing for agent authentication
- **Transport** — TCP, TLS, or in-process channels

## Agent Registration

```rust
use a2x_bus::{Bus, AgentFilter};
use a2x_core::agent_id::{AgentId, AgentType};

let bus = Bus::new();

// Register an agent
let id = AgentId::new(AgentType::CCS, "my-ccs-agent");
bus.register(id.clone(), capabilities)?;

// Discover agents
let all = bus.discover(&AgentFilter::All);
let ccs_only = bus.discover(&AgentFilter::ByType(AgentType::CCS));

// Send a message
bus.send_to(&target_id, packet)?;

// Broadcast
bus.broadcast(packet)?;
```

## Transport Layer

The Bus is generic over transport:

```rust
// In-process (for tests and simple setups)
let bus = Bus::<InProcessTransport>::new();

// TCP (for networked agents)
let bus = Bus::<TcpTransport>::new("127.0.0.1", 8000)?;

// TLS (for secure communication)
let bus = Bus::<TlsTransport>::new(config)?;
```

## Bus Bridge

The `BusBridge` connects two buses:

```rust
let bridge = BusBridge::new(bus_a, bus_b);
bridge.start()?;
// Messages published on bus_a are forwarded to bus_b and vice versa
```

## Identity & Signing

Agents can verify each other's identity via Ed25519:

```rust
let identity = AgentIdentity::generate();
let signed_message = identity.sign(&message)?;
let verified = identity.verify(&signed_message, &signature)?;
```
