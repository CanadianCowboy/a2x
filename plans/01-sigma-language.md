# A2X Σ∞ Language — Hyper-Symbolic ISA

> **The programming language itself — the instruction set architecture (ISA) that AIs write, read, and execute.**

---

## Table of Contents

1. [Overview](#1-overview)
2. [Instruction Format](#2-instruction-format)
3. [Intent Operators (Opcode)](#3-intent-operators-opcode)
4. [Context Operators (Operand)](#4-context-operators-operand)
5. [Plan Operators (Control Flow)](#5-plan-operators-control-flow)
6. [Data Operators (Immediate)](#6-data-operators-immediate)
7. [Programs are Packet Streams](#7-programs-are-packet-streams)
8. [Tokenizer Design](#8-tokenizer-design)
9. [Parser Design](#9-parser-design)
10. [SigmaProgram Type](#10-sigmaprogram-type)
11. [Program Composition](#11-program-composition)
12. [Program References](#12-program-references)
13. [Binary Instruction Encoding](#13-binary-instruction-encoding)

---

## 1. Overview

Σ∞ is the **programming language** that AIs write, read, and execute. Each packet is a **single instruction** in the AI's instruction set architecture (ISA). A program is a **sequence of Σ∞ packets** — a *packet stream* — that the CCS runtime executes sequentially, branching, or in parallel.

- **Crate:** `a2x-sigma`
- **Depends on:** `a2x-core`
- **Key files:** `tokenizer.rs`, `parser.rs`, `packet.rs`, `program.rs`, `intent.rs`, `context.rs`, `plan.rs`, `data.rs`

---

## 2. Instruction Format

### Text Form (Human-peekable)

```
⟦Σ∞⟧⟬I:<opcode> ∷ C:<operand/mem> ∷ P:<control_flow> ∷ D:<immediate>⟭
```

| Field | Role | Analogy |
|-------|------|---------|
| `⟦` `⟧` | Instruction boundary markers | `{ }` or `begin`/`end` |
| `Σ∞` | Language identifier | `.section .text` |
| `⟬` `⟭` | Agent execution context | CPU core ID or process context |
| `I` | **Opcode** — the instruction to execute | `MOV`, `ADD`, `JMP` |
| `C` | **Operand / Memory reference** — what to operate on | Register name, memory address |
| `P` | **Control flow** — how to sequence the next instruction | `JMP`, `CALL`, `RET`, `BR` |
| `D` | **Immediate data** — literal payload | Immediate value in `MOV R1, #42` |
| `∷` | Field separator | `,` in assembly |

### Binary Form (Compact, for transport)

See [Section 13](#13-binary-instruction-encoding) for full binary layout.

---

## 3. Intent Operators (Opcode)

Controls the **goal type, urgency, and mode** of the instruction.

| Symbol | Unicode | Name | Meaning |
|--------|---------|------|---------|
| `⚡` | U+26A1 | Lightning | Immediate execution |
| `⚠` | U+26A0 | Warning | Critical risk |
| `✦` | U+2726 | Star | Discovery/exploration |
| `✣` | U+2723 | Four-point star | Synthesis |
| `✕` | U+2715 | X mark | Cancel |
| `⟁` | U+27C1 | Reverse arrow | Contradiction |
| `⧖` | U+29D6 | Hourglass | Delay/hold |
| `⧗` | U+29D7 | Hourglass flow | Accelerate |
| `⩫` | U+2A6B | Parallel | Parallel multi-goal |
| `⩪` | U+2A6A | Merge | Merge goals |
| `⩨` | U+2A68 | Split | Split goals |

**Rust enum:**
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntentOp {
    Lightning,    // ⚡
    Warning,      // ⚠
    Star,         // ✦
    Synthesis,    // ✣
    Cancel,       // ✕
    Contradiction,// ⟁
    Delay,        // ⧖
    Accelerate,   // ⧗
    Parallel,     // ⩫
    Merge,        // ⩪
    Split,        // ⩨
}
```

---

## 4. Context Operators (Operand)

Controls the **world-state, uncertainty, scope, and memory reference**.

| Symbol | Unicode | Name | Meaning |
|--------|---------|------|---------|
| `⟘` | U+27D8 | Empty set | Null context |
| `⟙` | U+27D9 | Full set | Universal context |
| `⟚` | U+27DA | Compression | Compressed world-state |
| `⟞` | U+27DE | Wavy line | Uncertainty field |
| `⟡` | U+27E1 | Bowtie | Causal chain |
| `⟠` | U+27E0 | Diamond | Spatial chain |
| `⟢` | U+27E2 | Left arrow | Temporal chain |
| `⟣` | U+27E3 | Right arrow | Probabilistic context |
| `⟤` | U+27E4 | Double bar | Conflict context |
| `⟧` | U+27E7 | Corner | Resolved context |
| `⟨⟩` | U+27E8/9 | Angles | Context capsule (labels inside) |

**Addressing modes** (how memory is referenced):

| Mode | Syntax | Example | Description |
|------|--------|---------|-------------|
| **Direct** | `⟨label⟩` | `⟨sys⟩` | Look up a WorldGraph node by label |
| **Indirect** | `⟨id:NNN⟩` | `⟨id:42⟩` | Reference a node by its numeric ID |
| **Region** | `⟨.name⟩` | `⟨.goal⟩` | Reference a StateField region |
| **Query** | `⟨?pattern⟩` | `⟨?port:*⟩` | Graph query — matches nodes by pattern |
| **Relative** | `⟨-N⟩` | `⟨-1⟩` | Reference the N-th previous instruction's output |

---

## 5. Plan Operators (Control Flow)

Controls **how execution proceeds** — sequencing, branching, parallelism.

| Symbol | Unicode | Name | Meaning |
|--------|---------|------|---------|
| `⤈` | U+2908 | Down arrow bar | Descend into sub-plan |
| `⤉` | U+2909 | Up arrow bar | Ascend to meta-plan |
| `⤊` | U+290A | Up triangle | Escalate |
| `⤋` | U+290B | Down triangle | De-escalate |
| `⤐` | U+2910 | Branch fork | Branch |
| `⤑` | U+2911 | Merge fork | Merge |
| `⤒` | U+2912 | T-up | Enforce |
| `⤓` | U+2913 | T-down | Relax |
| `⥁` | U+2941 | Circle arrows | Parallel swarm |
| `⥂` | U+2942 | Right arrow | Sequential chain |
| `⥃` | U+2943 | Left arrow | Recursive |
| `⥄` | U+2944 | Double arrow | Self-modifying |

**Effect on VM instruction pointer (IP):**

| Plan Op | Effect |
|---------|--------|
| `⥂` (sequential) | IP += 1 |
| `⤐` (branch) | IP jumps to target address |
| `⤑` (merge) | IP = pop from call stack |
| `⤈` (descend) | Push IP+1, jump to sub-program |
| `⤉` (ascend) | Pop return address, jump there |
| `⥃` (recursive) | Push IP, jump to program start |
| `⥁` (swarm) | Fork N VM instances |
| `⥄` (self-modify) | Modify instruction stream, continue |

---

## 6. Data Operators (Immediate)

Controls the **payload type and structure** — what data the instruction carries.

| Symbol | Unicode | Name | Meaning |
|--------|---------|------|---------|
| `⌬` | U+232C | Benzene ring | Raw tensor block |
| `⌭` | U+232D | Circle | Latent vector block |
| `⌮` | U+232E | Integral | Graph delta |
| `⌯` | U+232F | Surface | Diff patch |
| `⌰` | U+2330 | Segment | Binary payload |
| `⌱` | U+2331 | Fuse | Multimodal fusion |
| `⌲` | U+2332 | Wave | Streaming block |
| `⌳` | U+2333 | Psi | Compressed summary |
| `⌴` | U+2334 | Anomaly | Anomaly payload |
| `⌵` | U+2335 | Tally marks | Structured schema |
| `⌶` | U+2336 | Greek letter | Self-describing payload |

---

## 7. Programs are Packet Streams

A program is a **sequence of instructions**:

```
⟦Σ∞⟧⟬I:✦ ∷ C:⟨scope⟩ ∷ P:⥂ ∷ D:⌵⟭          // explore scope
⟦Σ∞⟧⟬I:⚡✣ ∷ C:⟚⟨scope⟩ ∷ P:⥁⤈ ∷ D:⌬⟭     // immediate synthesis, parallel sub-plan
⟦Σ∞⟧⟬I:✣ ∷ C:⟨results⟩ ∷ P:⤑ ∷ D:⌳⟭        // merge results
⟦Σ∞⟧⟬I:✕ ∷ C:⟘ ∷ P:⤉ ∷ D:⟘⟭                // cancel, ascend
```

An LLM or orchestrator generates this stream. A CLI agent receives and executes it.

### Example: Scan system for anomalies

```
⟦Σ∞⟧⟬I:⚡✣⩫ ∷ C:⟚⟞⟨sys⟩ ∷ P:⥁⤒⤈ ∷ D:⌮⌳⌱⟭
```

Decoded: `BIND(IMMEDIATE, SYNTHESIZE, PARALLEL)` on compressed uncertain sys memory → enforce parallel sub-plan → store graph-delta + summary + fusion.

### Example: Report anomaly

```
⟦Σ∞⟧⟬I:⚠⟁ ∷ C:⟚⟤⟨22,8080⟩ ∷ P:⤊⥂ ∷ D:⌯⌴⟭
```

Decoded: `SIGNAL(CONTRA, CONFLICT)` on compressed conflict context `{22,8080}` → escalate sequential plan → emit diff-patch + anomaly-payload.

---

## 8. Tokenizer Design

Input: Raw string (Σ∞ text).
Output: `Vec<Token>`.

The lexer matches Unicode special characters against the operator tables using a trie-based matcher.

```rust
#[derive(Clone, Debug, PartialEq)]
/// Protocol identifier for the A2X instruction set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProtocolId {
    /// Σ∞ — hyper-symbolic ISA (default).
    SigmaInfinity,
    /// Ω — compiled latent representation.
    Omega,
    /// Raw binary payload.
    Raw,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    Boundary(BoundaryKind),      // ⟦ ⟧ ⟬ ⟭
    FieldSeparator,              // ∷
    IntentOp(IntentOp),
    ContextOp(ContextOp),
    PlanOp(PlanOp),
    DataOp(DataOp),
    Label(String),               // ⟨sys⟩ → "sys"
    ProtocolId(ProtocolId),       // Σ∞ or Ω
}

pub enum BoundaryKind { Open, Close }

fn lex(input: &str) -> Result<Vec<Token>, LexError> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        let token = match c {
            '⟦' => Token::Boundary(BoundaryKind::Open),
            '⟧' => Token::Boundary(BoundaryKind::Close),
            'Σ' if chars.peek() == Some(&'∞') => {
                chars.next(); // consume ∞
                Token::ProtocolId
            },
            '⚡' => Token::IntentOp(IntentOp::Lightning),
            // ... all other operators
            _ if c.is_alphanumeric() => {
                let mut label = String::from(c);
                while let Some(nc) = chars.peek() {
                    if nc.is_alphanumeric() || *nc == '_' || *nc == '-' {
                        label.push(chars.next().unwrap());
                    } else { break; }
                }
                Token::Label(label)
            },
            _ => return Err(LexError::UnknownCharacter(c)),
        };
        tokens.push(token);
    }
    Ok(tokens)
}
```

---

## 9. Parser Design

Input: `Vec<Token>`.
Output: `SigmaProgram` (instruction stream + label table).

```rust
fn parse(tokens: &[Token]) -> Result<SigmaProgram, ParseError> {
    let mut program = SigmaProgram::new();
    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i] {
            Token::Boundary(BoundaryKind::Open) => {
                let (packet, consumed) = parse_one_packet(&tokens[i..])?;
                program.push(packet);
                i += consumed;
            },
            Token::Label(name) => {
                program.labels.insert(name.clone(), program.len());
                i += 1;
            },
            _ => return Err(ParseError::UnexpectedToken { pos: i, token: tokens[i].clone() }),
        }
    }
    Ok(program)
}
```

### SigmaPacket Struct

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct SigmaPacket {
    pub boundary: Option<BoundaryPair>,
    pub protocol: ProtocolId,
    pub intent: IntentField,
    pub context: ContextField,
    pub plan: PlanField,
    pub data: DataField,
}
```

---

## 10. SigmaProgram Type

```rust
#[derive(Clone, Debug)]
pub struct SigmaProgram {
    /// Unique program ID (derived from Blake3 hash of contents).
    pub id: ProgramId,
    /// The instruction stream.
    pub instructions: Vec<SigmaPacket>,
    /// Symbol table: labels → instruction indices.
    pub labels: HashMap<String, usize>,
    /// Sub-programs (for ⤈ descend).
    pub sub_programs: HashMap<String, SigmaProgram>,
    /// Optional: original source / provenance metadata.
    pub metadata: ProgramMetadata,
}
```

Key methods:
- `new()` — create empty program
- `push(instruction)` — add an instruction
- `resolve_label(label) -> Option<usize>` — look up label
- `output() -> SigmaProgram` — extract result from last instruction's D field
- `compose(other)` — concatenate instruction streams
- `compute_id() -> ProgramId` — Blake3 hash of contents

---

## 11. Program Composition

| Operation | Method | Description |
|-----------|--------|-------------|
| **Sequence** | `program_a.compose(program_b)` | Append B's instructions after A's |
| **Branch** | Plan op `⤐` + label | Jump to a labeled sub-program |
| **Sub-plan** | Plan op `⤈` + label | Call a sub-program, return when done |
| **Parallel** | Plan op `⥁` + sub-program list | Fork N programs, run in parallel |
| **Merge** | Plan op `⤑` | Join parallel branches back together |

---

## 12. Program References

A program can reference another program by its `ProgramId` for caching and deduplication:

```rust
pub enum ProgramRef {
    /// Full program inline.
    Inline(SigmaProgram),
    /// Reference to a known program by its ID.
    /// The receiver must have it cached or request it.
    ById(ProgramId),
}
```

---

## 13. Binary Instruction Encoding

For efficient transport and VM decoding, each instruction has a compact binary form:

### Header Byte

```
 7   6   5   4   3   2   1   0
┌───┬───┬───┬───┬───┬───┬───┬───┐
│ Protocol │ Opcode     │ Flags │
└───┴───┴───┴───┴───┴───┴───┴───┘

Protocol: 2 bits — 00 = Σ∞, 01 = Ω, 10 = reserved, 11 = raw
Opcode:   4 bits — 0-15
Flags:    2 bits — 00 = normal, 01 = immediate, 10 = explore, 11 = safe
```

### Full Instruction Layout

```
┌──────────┬──────────┬──────────┬──────────┬──────────┐
│  Header  │  Operand │  Control │   Data   │ Checksum │
│  1 byte  │ 4 bytes  │ 2 bytes  │ variable │  4 bytes │
└──────────┴──────────┴──────────┴──────────┴──────────┘

Minimum: 11 bytes (without data payload)
```

### Operand Encoding

```
31              23              15              7             0
┌───────────────┬───────────────┬───────────────┬───────────────┐
│   Mode (2)    │           Target (30 bits)                   │
└───────────────┴───────────────────────────────────────────────┘

Mode (2 bits):
  00 = Label index
  01 = Direct NodeId
  10 = StateField region index
  11 = Immediate value (literal float/int)
```

### Control Flow Encoding

```
15              11              7             0
┌───────────────┬───────────────┬───────────────┐
│  Flow Op (4)  │  Target (12 bits)             │
└───────────────┴───────────────────────────────┘

Flow Ops:
  0000 = Sequential
  0001 = Jump absolute
  0010 = Jump relative
  0011 = Branch if true
  0100 = Branch if false
  0101 = Call
  0110 = Return
  0111 = Fork
  1000 = Merge
  1001 = Halt
```

---

*This sub-plan maps to phases 0–1 of the implementation roadmap.*
