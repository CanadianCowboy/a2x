// examples/01-sigma-hello.rs
// Basic Σ∞ program creation, parsing, and execution demo.
//
// Demonstrates:
//   - Creating Σ∞ packets with operator tables
//   - Building a SigmaProgram
//   - Parsing from text representation
//   - Executing on the CCS VM
//   - Inspecting the WorldGraph after execution
//
// Run: cargo run --example 01-sigma-hello

use a2x_ccs::CcsVm;
use a2x_core::graph::WorldGraph;
use a2x_core::memory::MemoryTrace;
use a2x_sigma::program::SigmaProgram;
use a2x_sigma::SigmaPacket;

fn main() {
    println!("=== A2X Σ∞ Hello World ===\n");

    // ── Step 1: Create a Σ∞ program from packets ──────────────────────────
    println!("Step 1 — Building a Σ∞ program from instructions:");
    let mut program = SigmaProgram::new();

    // Instruction 1: Explore context "hello" with sequential control flow
    let pkt1 = SigmaPacket::default(); // Simple NOP instruction
    program.push(pkt1);

    println!("  Program ID: {}", program.id);
    println!("  Instructions: {}", program.instructions.len());

    // ── Step 2: Parse a Σ∞ program from text ──────────────────────────────
    println!("\nStep 2 — Parsing Σ∞ from text:");
    let source = "⟦Σ∞⟧⟬I:✦ ∷ C:⟨world⟩ ∷ P:⥂ ∷ D:⌵⟭";
    match a2x_sigma::parse_program(source) {
        Ok(parsed) => {
            println!("  ✓ Parsed successfully");
            println!("  Program ID: {}", parsed.id);
            println!("  Instructions: {}", parsed.instructions.len());
            if let Some(first) = parsed.instructions.first() {
                println!(
                    "  First instruction: {}",
                    a2x_sigma::serialize_packet(first)
                );
            }
        }
        Err(e) => println!("  ✗ Parse error: {}", e),
    }

    // ── Step 3: Execute on the CCS VM ──────────────────────────────────────
    println!("\nStep 3 — Executing on the CCS VM:");
    let mut vm = CcsVm::new();
    vm.load(program.clone());

    println!("  WorldGraph nodes before: {}", vm.world_graph.node_count());
    println!("  Running VM...");
    match vm.run() {
        Ok(status) => println!("  ✓ VM finished with status: {:?}", status),
        Err(e) => println!("  ✗ VM error: {}", e),
    }
    println!("  WorldGraph nodes after: {}", vm.world_graph.node_count());
    println!("  MemoryTrace length: {}", vm.memory_trace.len());
    println!("  Uptime: {:?}", vm.uptime());

    // ── Step 4: Build a slightly more interesting program ─────────────────
    println!("\nStep 4 — A slightly more interesting program:");
    let source2 = "⟦Σ∞⟧⟬I:⚡✣ ∷ C:⟚⟨test⟩ ∷ P:⥂ ∷ D:⌵⟭";
    match a2x_sigma::parse_program(source2) {
        Ok(prog) => {
            let mut vm2 = CcsVm::new();
            vm2.load(prog);
            match vm2.run() {
                Ok(status) => {
                    println!("  ✓ Executed: {:?}", status);
                    println!("  WorldGraph nodes: {}", vm2.world_graph.node_count());
                    println!("  MemoryTrace entries: {}", vm2.memory_trace.len());
                }
                Err(e) => println!("  ✗ Error: {}", e),
            }
        }
        Err(e) => println!("  ✗ Parse error: {}", e),
    }

    println!("\n=== Demo complete ===");
}
