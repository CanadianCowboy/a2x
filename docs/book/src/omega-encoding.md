# Ω Tensor Encoding

Each Σ∞ instruction is encoded into a 29,796-element float tensor with four
distinct regions.

## Tensor Layout

```
┌──────────┬──────────┬──────────┬────────────────────────┐
│   I      │    C     │    P     │          D             │
│ 52 float │ 42 float │ 76 float │      29,626 float      │
│ (opcode) │(operand) │(control) │       (payload)        │
└──────────┴──────────┴──────────┴────────────────────────┘
                       29,796 total
```

## Region Details

### I-Region (52 floats) — Intent Encoding
Encodes the cognitive operator as a deterministic projection:
- 11 possible operators mapped to 52-dimensional space
- Uses Blake3 hashing to generate projection vectors
- Future: learned neural projection

### C-Region (42 floats) — Context Encoding
Encodes labels and references:
- Each label → 42-dimensional vector via Blake3
- Multiple labels → averaged projection
- Chain operators → directional encoding

### P-Region (76 floats) — Plan Encoding
Encodes control flow with 12 plan operators:
- Sequential (⥂) → identity projection
- Branch (⤐) → directional projection toward target

### D-Region (29,626 floats) — Data/Payload
Encodes the payload type and metadata:
- 11 data operator types
- CRC32 checksum for wire integrity
- Version byte for forward compatibility

## Serialization

```rust
use a2x_omega::OmegaProgram;

let omega: OmegaProgram = compile(sigma_program)?;
let bytes: Vec<u8> = omega.to_bytes()?;  // bincode serialization
let decoded: OmegaProgram = OmegaProgram::from_bytes(&bytes)?;
```

## Wire Format

The binary wire format includes:
- 4-byte magic number (`0x414F584F` = "AOXO")
- 4-byte CRC32 checksum
- Variable-length packed tensor data
