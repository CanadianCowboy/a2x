# TypeScript SDK

> ⏳ Planned — see `sdks/typescript/a2x-client.ts`.

## Current Status

The TypeScript SDK exists as a single-file module in the repository.

## Usage (Current)

```typescript
import { A2xClient } from './a2x-client';

const client = new A2xClient('http://localhost:8778');

// Execute a Σ∞ program
const result = await client.execute('⟦Σ∞⟧⟬I:✦ ∷ C:⟨hello⟩ ∷ P:⥂ ∷ D:⌬⟭');
console.log(result.result);
console.log(`Took ${result.execution_time_ms}ms`);
```

## Planned Features

- npm package (`npm install a2x-client`)
- Full TypeScript type definitions
- WebSocket streaming support
- Browser-compatible (no Node.js requirement)
- Deno support
