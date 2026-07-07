# A2X — Expansion Roadmap

> Beyond the Team Briefing. Growing A2X into a visible, usable, impressive project.

---

## 🚀 Expansion Ideas

### 1. Web Dashboard 🖥️ _(in progress)_

**Live visualization of the entire A2X system in a browser.**

- WorldGraph force-directed graph (nodes = concepts, edges = relations)
- StateField heatmaps (tensor regions as color grids)
- Agent status cards (Orchestrator, CLI, CCS, LLM — online/offline/load)
- Bus traffic log (real-time scrolling terminal of Σ∞ packets)
- Σ∞ program playground (type a program, execute it, see results)
- Breakpoint + single-step probe UI

**Stack:** Single HTML file served by the gateway (`GET /`), WebSocket for real-time updates. No build tooling, no npm.

**Status:** 🔨 Building now

---

### 2. Real Python SDK 🐍

**Turn `sdks/python/a2x_client.py` into a pip-installable package.**

- `pip install a2x-client`
- Connect to the gateway, send Σ∞ programs, stream results
- Async support via `aiohttp`
- Type hints, docstrings, `py.typed`
- Publish to PyPI

**Status:** ⏳ Planned

---

### 3. `a2x` CLI Polish 🛠️

**Make the CLI a joy to use.**

- `a2x shell` — interactive REPL for Σ∞ programs (colored output, tab completion)
- `a2x monitor` — live bus traffic viewer in the terminal (like `htop` for A2X)
- `a2x probe --visualize` — ASCII art WorldGraph + StateField in terminal
- `a2x dashboard` — one command to launch the web dashboard

**Status:** ⏳ Planned

---

### 4. WASM Runtime 🌐

**Compile the CCS VM to WebAssembly — A2X programs run in browsers.**

- `a2x-ccs` compiles to `wasm32-unknown-unknown`
- `a2x-wasm` crate with JS bindings
- Demo: Σ∞ program editor → compile → execute in-browser
- Potential: A2X as a browser-based AI orchestration language

**Status:** ⏳ Planned (see plans/15-wasm.md)

---

### 5. Doc Site + Examples 📚

**Make A2X learnable.**

- mdBook documentation site (`docs.a2x.dev`)
- Tutorial: "Your first Σ∞ program"
- Operator reference (all 40+ symbols with examples)
- API reference for every crate
- Gallery of Σ∞ programs that do real things
- Architecture deep-dives (CCS VM, Ω compilation)

**Status:** ⏳ Planned

---

### 6. Benchmark Suite 📊

**Prove A2X is fast.**

- Criterion benchmarks for every layer:
  - Tokenizer throughput (> 1M packets/sec)
  - Parser throughput (> 500K packets/sec)
  - Ω encode/decode (> 5M packets/sec)
  - Bus message routing (< 100µs latency)
  - WorldGraph query (< 1µs per neighbor)
- Performance dashboard (HTML report)
- CI integration (regression detection)

**Status:** ⏳ Planned (a2x-sigma already has benches/)

---

## 📋 Prioritized Order

| # | Idea | Why First |
|:-:|------|-----------|
| 1 | **Web Dashboard** | Makes everything tangible. One command, open browser, see A2X live. Highest "wow" factor. |
| 2 | CLI Polish | Complements the dashboard — terminal-native power users |
| 3 | Doc Site | Needed before anyone outside this repo can use A2X |
| 4 | Python SDK | Opens A2X to the Python ecosystem (data science, ML, web) |
| 5 | Benchmarks | Prove performance, catch regressions |
| 6 | WASM | Ambitious — browser-native A2X execution |

---

*ColdStart Intelligence Labs — Precision. Clarity. Operator-Grade.*
