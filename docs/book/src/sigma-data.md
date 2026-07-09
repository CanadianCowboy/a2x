# Data Operators

The **D-field** specifies the payload type for a packet.

## Payload Encodings

| Symbol | Name | Description |
|--------|------|-------------|
| ⌬ | **RawTensor** | Raw tensor data (f32 array) |
| ⌭ | **LatentVector** | Latent space vector representation |
| ⌮ | **GraphDelta** | WorldGraph delta (node/edge changes) |
| ⌯ | **DiffPatch** | Differentiation patch |
| ⌰ | **Binary** | Binary payload |
| ⌱ | **Fusion** | Fused operator data |
| ⌲ | **Streaming** | Stream marker |
| ⊗ | **Product** | Cross-product of two vectors |
| ⊕ | **Sum** | Element-wise sum |
| ⊖ | **Difference** | Element-wise difference |
| ⊘ | **Norm** | Normalized vector |

## Usage

### Raw Tensor (⌬)
The default — carries an f32 tensor:
```text
D:⌬
```

### Graph Delta (⌮)
Encodes changes to the WorldGraph:
```text
D:⌮
```
Used by BIND and DIFFERENTIATE to record node/edge mutations.

### Latent Vector (⌭)
Carries a latent space representation:
```text
D:⌭
```
Used in the Ω pipeline for learned encodings.

### Streaming (⌲)
Marks a packet as part of a stream:
```text
D:⌲
```
The VM processes streaming packets without waiting for completion.

## Data Flow in the Compiler

```
Σ∞ D-field → Omega compiler → 29,796-element float tensor
                                  ↓
                             4 regions:
                             - I: 52 floats (opcode)
                             - C: 42 floats (operand/label)
                             - P: 76 floats (control flow)
                             - D: 29,626 floats (payload)
```
