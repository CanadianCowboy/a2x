# Quick Start

Get A2X running in under 2 minutes.

## Prerequisites

- **Rust** (1.75+) — [rustup.rs](https://rustup.rs)
- **Ollama** (optional, for ChatAgent) — [ollama.com](https://ollama.com)

## Install

```bash
# Clone the repository
git clone https://github.com/CanadianCowboy/a2x.git
cd a2x

# Build everything
cargo build --release

# Verify it works
./target/release/a2x --version
./target/release/a2x --help
```

## Launch the Dashboard

```bash
# One command — opens the web dashboard in your browser
./target/release/a2x dashboard
```

Visit `http://localhost:8778` to see:
- **Agent cards** — live status of all running agents
- **WorldGraph** — force-directed graph of concepts and relations
- **Heatmap** — StateField region visualization
- **Chat tab** — talk to the ChatAgent (requires Ollama or OpenAI key)

## Run Your First Σ∞ Program

```bash
# Parse a Σ∞ program
./target/release/a2x parse "⟦Σ∞⟧⟬I:✦ ∷ C:⟨hello⟩ ∷ P:⥂ ∷ D:⌬⟭"

# Run it
./target/release/a2x run "⟦Σ∞⟧⟬I:✦ ∷ C:⟨hello⟩ ∷ P:⥂ ∷ D:⌬⟭"

# Interactive REPL
./target/release/a2x shell
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `A2X_CHAT_BACKEND` | `ollama` | LLM backend (`ollama` or `openai`) |
| `A2X_CHAT_MODEL` | `llama3.2` | Model name |
| `A2X_OPENAI_API_KEY` | — | OpenAI API key (if using OpenAI backend) |
| `A2X_OLLAMA_HOST` | `http://localhost:11434` | Ollama server URL |
| `A2X_BUS_HOST` | `127.0.0.1` | Bus listen address |
| `A2X_BUS_PORT` | `0` (auto) | Bus listen port |
| `A2X_GATEWAY_HOST` | `127.0.0.1` | Gateway listen address |
| `A2X_GATEWAY_PORT` | `8778` | Gateway listen port |
