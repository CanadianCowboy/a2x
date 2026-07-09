# Gateway & Dashboard

The Entity Gateway is the external interface for A2X. It provides HTTP,
WebSocket, TCP, and stdio listeners for external applications.

## Gateway Daemon

```bash
a2x-gatewayd
```

Reads configuration from environment variables:
- `A2X_GATEWAY_HOST` — listen address
- `A2X_GATEWAY_PORT` — listen port
- `A2X_CHAT_BACKEND` — LLM backend
- `A2X_CHAT_MODEL` — model name
- `A2X_OPENAI_API_KEY` — OpenAI key (if using OpenAI)

## Listeners

| Protocol | Use Case |
|----------|----------|
| **HTTP** | REST API for program execution, entity management |
| **WebSocket** | Real-time dashboard updates, streaming responses |
| **TCP** | High-performance agent communication |
| **stdio** | Local subprocess integration |

## HTTP API

### Execute a Program

```http
POST /a2x/execute
Content-Type: application/json

{
  "program": "⟦Σ∞⟧⟬I:✦ ∷ C:⟨test⟩ ∷ P:⥂ ∷ D:⌬⟭",
  "entity_id": "optional-auth-token"
}
```

Response:
```json
{
  "result": "GROUND ⟨test⟩ — concept allocated (node #7)",
  "execution_time_ms": 2
}
```

### WebSocket Dashboard

```
ws://localhost:8778/ws
```

Receives snapshots every 500ms:
```json
{
  "tick": 1234,
  "entities": [...],
  "agent_count": 3,
  "bus_events": [...]
}
```

## Authentication

| Method | Use Case |
|--------|----------|
| `API key` | Service-to-service authentication |
| `JWT` | User session authentication |
| `Local` | Development (no auth required) |

## Rate Limiting

Token bucket rate limiter per entity:
- Default: 100 requests/second
- Burst: 200 requests
- Configurable per entity type
