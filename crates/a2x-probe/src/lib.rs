// a2x-probe — Probe/debug tools for inspecting CCS internals
// See plans/07-probe.md
//
// Stub crate — to be implemented in Phase 5.

pub fn stub() -> &'static str {
    "a2x-probe stub"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        assert_eq!(stub(), "a2x-probe stub");
    }
}
