# Startup & Shutdown

A2X provides a managed boot and shutdown system for production deployments.

## Boot Sequence

```
1. Load configuration (environment + config file)
2. Initialize secure storage
3. Load persistent state (agents, WorldGraph)
4. Set up key rotation schedule
5. Register built-in agents on the bus
6. Bootstrap WorldGraph (12 concepts + 10 edges)
7. Start gateway listeners (HTTP, WS, TCP)
8. Begin resilience monitoring
```

## Shutdown Sequence

```
1. Receive shutdown signal (SIGTERM, SIGINT, Ctrl+C)
2. Drain active connections (grace period)
3. Persist agent state to disk
4. Unregister agents from the bus
5. Flush and close secure storage
6. Exit with code 0
```

## Configuration

```rust
use a2x_startup::{BootConfig, boot};

let config = BootConfig::from_env()?;
let runtime = boot(config).await?;
```

Configuration sources (in priority order):
1. Command-line arguments
2. Environment variables (`A2X_*` prefix)
3. Config file (`~/.a2x/config.toml`)
4. Defaults

## Persistence

Agent state is persisted to `~/.a2x/state/`:
- `agents.json` — agent registry
- `worldgraph.bin` — WorldGraph snapshot
- `conversations/` — ChatAgent conversation history

## Key Rotation

Ed25519 signing keys are automatically rotated:
- Default interval: 24 hours
- Grace period: 1 hour (old key still valid)
- Rotation is atomic (no window of invalidity)

## Resilience

Built-in resilience features:
- Automatic agent restart on crash (configurable retry)
- Gateway connection draining (10 second grace period)
- State persistence before shutdown
- Health check endpoint (`GET /health`)
