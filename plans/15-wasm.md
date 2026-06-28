# A2X WASM Support — Browser-Based Agents & Web Dashboards

> **Running A2X in the browser via WebAssembly — for web dashboards, browser-based agents, and edge execution.**

---

## 1. Overview

WebAssembly (WASM) extends A2X to run in browsers, edge runtimes, and other constrained environments. This enables:

| Use Case | Why WASM | Component |
|----------|----------|-----------|
| **Web dashboard** | Real-time probe visualization | `a2x-probe` + WebSocket |
| **Browser agent** | Client-side A2X execution without a server | Lightweight WASM CCS VM |
| **Edge function** | A2X programs on CDN edge (Cloudflare Workers) | `a2x-bus` WebSocket transport |
| **Plugin system** | Third-party agents as WASM modules | WASM-based custom opcodes |

---

## 2. What CAN Run in WASM

| Crate | WASM Support | Notes |
|-------|:-----------:|-------|
| `a2x-core` | ✅ Full | No platform dependencies |
| `a2x-sigma` | ✅ Full | Pure string parsing |
| `a2x-omega` | ⚠️ Partial | `ndarray` not WASM-ready; use `no_std` |
| `a2x-ccs` | ⚠️ Partial | `petgraph` works; skip file I/O |
| `a2x-bus` | ✅ WebSocket | In-memory + WebSocket transport only |
| `a2x-agents` | ⚠️ Partial | No CLI agent (no shell in browser) |
| `a2x-probe` | ✅ Full | Visualization is WASM-native |
| `a2x-cli` | ❌ N/A | No CLI in browser |
| `a2x-gateway` | ❌ N/A | Needs network listeners |
| `a2x-client` | ✅ Full | HTTP + WebSocket to gateway |

---

## 3. WASM Target Setup

### Cargo Configuration

```toml
# In each crate's Cargo.toml (conditional compilation)
[lib]
crate-type = ["cdylib", "rlib"]  # cdylib for WASM

[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen = "0.2"
web-sys = { version = "0.3", features = ["WebSocket", "console"] }
```

### Crate Feature Gates

```toml
# a2x-core/Cargo.toml
[features]
default = ["std"]
std = []
wasm = ["wasm-bindgen"]          # WASM-specific bindings

# a2x-ccs/Cargo.toml
[features]
default = ["std"]
std = []
wasm = []                         # Skip file I/O, use in-memory storage
```

---

## 4. WASM-Specific Components

### WASM CCS VM (Lightweight)

For browser execution, the CCS VM is stripped to essentials:

```rust
/// WASM-compatible CCS VM — no file I/O, no process execution.
pub struct WasmCcsVm {
    world_graph: WorldGraph,
    state_field: StateField,
    program: SigmaProgram,
    ip: usize,
    // No call stack in minimal mode (sequential only)
    // No file I/O (WorldGraph saved to IndexedDB instead)
}
```

### IndexedDB Storage

```rust
/// WASM storage backend using IndexedDB.
pub struct WasmStorage {
    db: IdbDatabase,
}

impl WasmStorage {
    pub async fn open(name: &str) -> Result<Self, WasmError> {
        // Open IndexedDB database
    }

    pub async fn save_world_graph(&self, data: &[u8]) -> Result<(), WasmError> {
        // Save to IndexedDB object store
    }

    pub async fn load_world_graph(&self) -> Result<Option<Vec<u8>>, WasmError> {
        // Load from IndexedDB
    }
}
```

### WebSocket Transport

```rust
/// WASM-compatible WebSocket transport.
pub struct WasmWsTransport {
    ws: WebSocket,
}

impl Transport for WasmWsTransport {
    async fn connect(&self, addr: SocketAddr) -> Result<Box<dyn Connection>, TransportError> {
        // Use web-sys WebSocket API
    }
}
```

---

## 5. Web Dashboard Architecture

```html
<!-- Browser connects to A2X via gateway WebSocket + probe channel -->
┌─────────────────────────────────────┐
│         Browser (WASM)              │
│                                     │
│  ┌──────────┐    ┌───────────────┐  │
│  │ Dashboard │◄───│ WASM Probe    │  │
│  │ (HTML/JS) │    │ Client        │  │
│  └──────────┘    └───────┬───────┘  │
│                          │          │
└──────────────────────────┼──────────┘
                           │ WebSocket
                           ▼
                 ┌──────────────────┐
                 │  A2X Gateway     │
                 │  (:8779 WS)      │
                 └────────┬─────────┘
                          │
                          ▼
                 ┌──────────────────┐
                 │  Agent (CCS VM)  │
                 └──────────────────┘
```

### Dashboard Features (WASM)

- **Live probe data** — WebSocket streams ProbeSnapshot events
- **WorldGraph visualization** — WASM-rendered graph (egui or custom canvas)
- **StateField heatmap** — Live tensor region heatmap
- **Instruction tracer** — Step-through debugger with WASM keyboard shortcuts
- **Program editor** — Monaco or CodeMirror with Σ∞ syntax highlighting

---

## 6. NPM Package (Future)

For JavaScript/TypeScript integration:

```
a2x-client-wasm/
├── pkg/           # WASM + JS bindings (wasm-pack output)
├── src/
│   ├── lib.rs     # Rust → WASM entry point
│   └── bindings/  # wasm-bindgen interop
├── package.json
└── README.md
```

```javascript
// JavaScript usage
import { A2XClient, SigmaProgram } from 'a2x-client-wasm';

const client = new A2XClient('ws://localhost:8779/a2x/stream');
await client.connect();

// Execute a program
const result = await client.execute(`
    ⟦Σ∞⟧⟬I:⚡✣ ∷ C:⟨sys⟩ ∷ P:⥂ ∷ D:⌳⟭
`, { timeout: 5000 });

// Stream probe data
const probeStream = client.probe('cli-1');
for await (const snapshot of probeStream) {
    console.log('WorldGraph state:', snapshot);
}
```

---

## 7. WASM Build Pipeline

```yaml
# .github/workflows/wasm.yml
name: WASM Build

on:
  push:
    branches: [main]

jobs:
  build-wasm:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: rustup target add wasm32-unknown-unknown
      - run: cargo build --target wasm32-unknown-unknown -p a2x-core -p a2x-sigma -p a2x-probe
      - run: cargo install wasm-pack
      - run: wasm-pack build crates/a2x-client --target web
      - uses: actions/upload-artifact@v3
        with:
          name: a2x-client-wasm
          path: crates/a2x-client/pkg/
```

---

## 8. WASM Limitations

| Feature | Limitation | Workaround |
|---------|-----------|------------|
| File I/O | No filesystem | IndexedDB |
| Network | No raw TCP | WebSocket only |
| Process execution | No shell | N/A (browser) |
| Threading | WASM is single-threaded | Web Workers for concurrency |
| SIMD | WASM SIMD limited | Fallback to scalar ops |
| GPU | WebGPU experimental | Use `candle` JS backend instead |

---

*This sub-plan maps to Phase 5 of the implementation roadmap.*
