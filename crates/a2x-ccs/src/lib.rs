// a2x-ccs — CryoCore Cognitive Substrate runtime VM
//
// See plans/03-ccs-vm.md for the full design specification.
//
// This crate implements:
// - WorldGraph: petgraph-backed persistent graph memory (heap)
// - StateField: Vec<f32>-backed working memory (registers)
// - MemoryTrace: vec-backed execution history
// - PolicyField: stub policy (Phase 0)
// - CcsVm: fetch-decode-execute loop with control flow
// - SafetyConstraints: opcode allowlisting, bounds checking
// - Operators: bind, differentiate, ground, evolve, reflect, plan, actuate (stubs)
// - Probe: debug/inspection interface stubs

// Public modules
pub mod error;
pub mod memory;
pub mod operators;
pub mod policy;
pub mod probe;
pub mod safety;
pub mod state;
#[cfg(feature = "ndarray")]
pub mod state_ndarray;
pub mod vm;
pub mod world_graph;

// Re-export commonly used types at crate root
pub use error::{StateError, VmError, WorldGraphError};
pub use memory::VecMemoryTrace;
pub use operators::actuate::ExternalCommand;
pub use operators::bind::bind;
pub use operators::differentiate::differentiate;
pub use operators::ground::ground;
pub use operators::plan::{plan, Action};
pub use operators::reflect::{reflect, PolicyUpdate};
pub use policy::StubPolicy;
pub use probe::{
    BreakpointType, ProbeEvent, ProbeQuery, ProbeSnapshot, ProbeTraceEntry, TracerMode,
};
pub use safety::{SafetyClassification, SafetyConstraints, SafetyLevel};
pub use state::{init_default_regions, FlatStateField, StateRegion};
#[cfg(feature = "ndarray")]
pub use state_ndarray::{init_ndarray_default_regions, NdArrayStateField};
pub use vm::{CcsVm, VmLimits, VmStatus};
pub use world_graph::PetgraphWorldGraph;
