# Installation

## Prerequisites

A2X requires **Rust 1.75 or later**. Install via [rustup](https://rustup.rs):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Building from Source

```bash
git clone https://github.com/CanadianCowboy/a2x.git
cd a2x
cargo build --release
```

The binary will be at `target/release/a2x`.

## Optional: Ollama for ChatAgent

To use the ChatAgent with local LLM support:

```bash
# Install Ollama
curl -fsSL https://ollama.com/install.sh | sh

# Pull a model
ollama pull llama3.2
```

## Optional: OpenAI

Set your API key:

```bash
export A2X_CHAT_BACKEND=openai
export A2X_CHAT_MODEL=gpt-4o-mini
export A2X_OPENAI_API_KEY=sk-your-key-here
```

## Verify Installation

```bash
a2x --version
# a2x 0.9.0-alpha

a2x --help
# Shows all available subcommands
```
