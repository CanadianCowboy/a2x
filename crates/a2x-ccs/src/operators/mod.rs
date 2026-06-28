// CCS VM operators — Phase 0 stubs.
// See plans/03-ccs-vm.md §4 and plans/02-omega-compiler.md Appendix A.

// Each operator module implements a specific CCS primitive operation.
// Phase 0: stub implementations that return basic results.
// Phase 2+: full neural implementations with GPU acceleration.

pub mod actuate;
pub mod bind;
pub mod differentiate;
pub mod evolve;
pub mod ground;
pub mod plan;
pub mod reflect;
