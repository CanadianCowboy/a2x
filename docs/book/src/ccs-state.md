# State Field

The StateField stores typed regions of state — beliefs, goals, memory, and
general-purpose regions.

## Regions

| Region | Type | Purpose |
|--------|------|---------|
| **belief** | f32 vector | Current belief state |
| **goal** | f32 vector | Target/desired state |
| **memory** | f32 vector | Compressed memory trace |
| `region_0` – `region_7` | f32 vector | General-purpose regions |

## Region API

```rust
use a2x_ccs::state::StateField;

let mut state = StateField::default();

// Write to a region
state.write("belief", &data)?;

// Read from a region
let belief = state.read("belief")?;

// Clear a region
state.clear("memory")?;
```

## Region Stats

The StateField provides per-region statistics:

```rust
let stats = state.region_stats("belief")?;
println!("min={} max={} mean={} sum={}", stats.min, stats.max, stats.mean, stats.sum);
```

These stats power the dashboard heatmap — each region tile is color-coded by
its activation level (blue=cold, red=hot).

## ndarray Backend

The StateField is backed by `ndarray` for efficient tensor operations:

- O(1) read/write access
- BLAS-accelerated math (when available)
- Memory-efficient zero-copy views

## Heatmap Visualization

In the web dashboard, the StateField is visualized as a color-coded grid:

- **Blue** — low activation (dormant regions)
- **Green** — moderate activation
- **Yellow** — high activation
- **Red** — maximum activation (hot path)
