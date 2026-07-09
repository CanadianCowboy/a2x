# Python SDK

> ⏳ Planned — not yet pip-installable. See `sdks/python/a2x_client.py`.

## Current Status

The Python SDK exists as a single-file module in the repository. Future work
will make it pip-installable.

## Usage (Current)

```python
from a2x_client import A2xClient

client = A2xClient("http://localhost:8778")

# Execute a Σ∞ program
result = client.execute('⟦Σ∞⟧⟬I:✦ ∷ C:⟨hello⟩ ∷ P:⥂ ∷ D:⌬⟭')
print(result["result"])
print(f"Took {result['execution_time_ms']}ms")

# Stream results
for chunk in client.execute_stream(program):
    print(chunk)
```

## Planned Features

- `pip install a2x-client`
- Async support via `aiohttp`
- Full type hints and docstrings
- PyPI publication
- Jupyter notebook integration

## Installation (Planned)

```bash
pip install a2x-client
```
