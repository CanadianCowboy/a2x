# A2X Entity Gateway — Connecting Anything to A2X

> **The bridge between external entities (apps, APIs, humans, systems) and the A2X ecosystem. The "Anything" in Agent-to-Anything.**

---

## 1. Overview

An **entity** is any external system or user that communicates with A2X. Entities do not run a CCS VM natively — they connect through the **gateway**, which translates between the entity's native protocol and A2X bus messages.

| Entity Type | Native Protocol | Adapter |
|-------------|----------------|---------|
| Human (CLI) | stdin/stdout | TUI → Σ∞ packet stream |
| Human (Web) | HTTP / WebSocket | REST/WS → A2X bus |
| AI (LLM) | HTTP (OpenAI format) | API call → Σ∞ program |
| Existing app | gRPC / REST | Protocol bridge |
| Database | SQL / custom | Query → WorldGraph lookup |
| Robot | ROS / serial | Sensor stream → Σ∞ perception |
| CI/CD | Webhook / REST API | Event → trigger program |

- **Crates:** `a2x-gateway`, `a2x-client`, `a2x-entity-http`, `a2x-entity-ws`, `a2x-entity-tcp`, `a2x-entity-stdio`
- **Depends on:** `a2x-bus`, `a2x-sigma`, `a2x-core`

---

## 2. Architecture

```
┌─────────────────────────────────────────────┐
│               A2X BUS                       │
└────────────────┬────────────────────────────┘
                 │
         ┌───────┴───────┐
         │  A2X GATEWAY   │
         │(entity bridge) │
         └───────┬───────┘
                 │
    ┌────────────┼────────────┐
    │            │            │
    ▼            ▼            ▼
┌────────┐ ┌──────────┐ ┌──────────┐
│ Entity │ │ Entity   │ │ Entity   │
│ (CLI)  │ │ (HTTP)   │ │ (DB)     │
└────────┘ └──────────┘ └──────────┘
```

---

## 3. Entity Trait

```rust
#[async_trait]
pub trait Entity: Send + Sync {
    fn entity_id(&self) -> EntityId;
    fn entity_type(&self) -> EntityType;
    fn display_name(&self) -> String;
    async fn send(&self, program: &Packet) -> Result<(), EntityError>;
    async fn recv(&self) -> Result<Packet, EntityError>;
    fn is_alive(&self) -> bool;
    fn capabilities(&self) -> Vec<Capability>;
}
```

---

## 4. Gateway Service

```rust
pub struct Gateway {
    entities: HashMap<EntityId, Box<dyn Entity>>,
    bus: BusConnection,
    listeners: Vec<Box<dyn ProtocolListener>>,
}

impl Gateway {
    pub async fn start(&mut self) -> Result<(), GatewayError> {
        for listener in &self.listeners { listener.start().await?; }
        loop {
            tokio::select! {
                entity = accept_entity() => self.register_entity(entity).await?,
                Some((entity_id, program)) = self.bus.recv_for_entity() => {
                    if let Some(entity) = self.entities.get(&entity_id) {
                        entity.send(&program).await?;
                    }
                },
            }
        }
    }
}
```

---

## 5. Protocol Listeners

### HTTP/REST Listener

```
POST /a2x/execute
Content-Type: application/json
{
  "program": "⟦Σ∞⟧⟬I:⚡✣⩫...⟭",
  "format": "sigma",
  "timeout_ms": 5000
}

Response 200:
{
  "result": "⟦Σ∞⟧⟬I:⚠⟁...⟭",
  "execution_time_ms": 234,
  "status": "completed"
}
```

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/a2x/execute` | Execute a Σ∞/Ω program |
| `GET` | `/a2x/entities` | List connected entities/agents |
| `GET` | `/a2x/entities/{id}` | Get entity/agent details |
| `GET` | `/a2x/probe/{agent_id}` | Probe agent state |
| `POST` | `/a2x/stream` | WebSocket upgrade |
| `POST` | `/a2x/webhook` | Register webhook callback |

### WebSocket Listener

For streaming, bidirectional A2X communication:
```
Client sends:   ⟦Σ∞⟧⟬I:⚡✦ ∷ C:⟨log⟩ ∷ P:⥂ ∷ D:⌲⟭    // stream logs
Server streams: ⟦Σ∞⟧⟬I:⟁ ∷ C:⟨log⟩ ∷ D:∂⟨[line 1]⟩⟭
Server streams: ⟦Σ∞⟧⟬I:⟁ ∷ C:⟨log⟩ ∷ D:∂⟨[line 2]⟩⟭
Server streams: ⟦Σ∞⟧⟬I:⤑ ∷ C:⟨log⟩ ∷ D:⌳⟨summary⟩⟭  // merged result
```

### TCP Listener

Raw socket, length-prefixed packets: `[4-byte length][serialized bytes]`

### stdin/stdout Listener

For CLI/pipe integration:
```bash
echo "⟦Σ∞⟧⟬I:✦ ∷ C:⟨sys⟩ ∷ P:⥂ ∷ D:⌵⟭" | a2x-gateway --listen stdio
```

### Webhook Callback

Instead of polling, entities register a URL for async results:
```
POST /my-service/a2x-callback
Content-Type: application/json
{
  "correlation_id": "abc-123",
  "result": "⟦Σ∞⟧⟬I:⚠⟁...⟭",
  "status": "completed"
}
```

---

## 6. Authentication

```rust
pub enum AuthMethod {
    ApiKey(String),        // X-A2X-Key header
    BearerToken(String),   // JWT
    ClientCert(Vec<u8>),   // TLS client cert
    Local,                 // Unix socket, no auth
}

pub struct EntityPermissions {
    pub entity_id: EntityId,
    pub max_instructions: u64,
    pub allowed_opcodes: Option<Vec<Opcode>>,
    pub allowed_modes: Vec<AddressingMode>,
    pub can_probe: bool,
    pub can_network: bool,
    pub rate_limit: u32,
}
```

---

## 7. Client SDKs

### Rust (`a2x-client`)

```rust
pub struct A2xClient {
    gateway_url: String,
    api_key: String,
    client: reqwest::Client,
}

impl A2xClient {
    pub fn new(gateway_url: &str, api_key: &str) -> Self;
    pub async fn execute(&self, program: SigmaProgram) -> Result<SigmaProgram, ClientError>;
    pub async fn stream(&self) -> Result<A2xStream, ClientError>;
    pub async fn register_webhook(&self, url: &str) -> Result<(), ClientError>;
    pub async fn list_entities(&self) -> Result<Vec<EntityInfo>, ClientError>;
}
```

### Python (third-party starter)

```python
class A2X:
    def __init__(self, gateway_url: str, api_key: str): ...
    def execute(self, program: str, timeout: int = 5000) -> str: ...
    def stream(self) -> WebSocket: ...
```

### JavaScript (third-party starter)

```javascript
class A2X {
    constructor(gatewayUrl, apiKey) { ... }
    async execute(program, timeoutMs = 5000) { ... }
    stream() { ... }
}
```

---

## 8. Configuration

```toml
# ~/.a2x/gateway.toml
[gateway]
bind_address = "0.0.0.0:8777"

[http]
enabled = true;  port = 8778

[websocket]
enabled = true;  port = 8779

[tcp]
enabled = true;  port = 8780

[stdio]
enabled = true

[auth]
mode = "api_key"
api_keys = [
    { key = "sk-a2x-abc123", permissions = "admin" },
]

[webhook]
enabled = true;  timeout_ms = 10000;  max_retries = 3
```

---

## 9. Official Entity Crates

| Crate | Purpose |
|-------|---------|
| `a2x-gateway` | Core gateway: entity registry, auth, routing |
| `a2x-client` | Rust client SDK |
| `a2x-entity-http` | HTTP/REST protocol listener + webhook |
| `a2x-entity-ws` | WebSocket protocol listener |
| `a2x-entity-tcp` | Raw TCP protocol listener |
| `a2x-entity-stdio` | stdin/stdout listener |

---

*This sub-plan maps to Phase 6 of the implementation roadmap.*
