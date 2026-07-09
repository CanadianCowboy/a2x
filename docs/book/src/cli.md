# CLI Reference

The `a2x` command-line interface provides access to all A2X functionality.

## Commands

```bash
a2x [SUBCOMMAND]
```

| Command | Description |
|---------|-------------|
| `a2x dashboard` | Launch web dashboard + gateway on `localhost:8778` |
| `a2x shell` | Interactive Σ∞ REPL with colored output |
| `a2x monitor` | Live bus state viewer |
| `a2x parse <program>` | Parse a Σ∞ program and show its structure |
| `a2x run <program>` | Parse and execute a Σ∞ program |
| `a2x agents` | List running agents on the bus |
| `a2x probe` | Attach probe to a running agent |
| `a2x help` | Show help for any command |

## Dashboard

```bash
a2x dashboard
```

One-command launch:
- Starts the gateway with HTTP/WS listeners
- Registers built-in agents (Orchestrator, CCS, ChatAgent)
- Bootstraps the WorldGraph with foundational concepts
- Opens browser to `http://localhost:8778`

Environment variables respected:
- `A2X_CHAT_BACKEND` — LLM backend (default: `ollama`)
- `A2X_CHAT_MODEL` — model name (default: `llama3.2`)
- `A2X_GATEWAY_HOST` — listen address (default: `127.0.0.1`)
- `A2X_GATEWAY_PORT` — listen port (default: `8778`)

## Shell

```bash
a2x shell
```

Interactive REPL features:
- Direct Σ∞ program typing and execution
- `:parse <program>` — parse without executing
- `:agents` — list all bus-registered agents
- `:help` — show available commands
- `:exit` or `Ctrl+D` — quit

## Monitor

```bash
a2x monitor
```

Shows live bus state:
- Agent listing with types and capabilities
- Capability matrix
- Demo dispatch to each agent

## Parse / Run

```bash
a2x parse "⟦Σ∞⟧⟬I:✦ ∷ C:⟨hello⟩ ∷ P:⥂ ∷ D:⌬⟭"
a2x run "⟦Σ∞⟧⟬I:✦ ∷ C:⟨hello⟩ ∷ P:⥂ ∷ D:⌬⟭"
```

Parse shows the tokenized packet structure. Run executes it on a local CCS VM.
