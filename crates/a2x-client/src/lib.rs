// a2x-client — Rust client SDK for connecting to A2X
// See plans/06-entity-gateway.md
//
// Stub crate — to be implemented in Phase 6.

pub fn stub() -> &'static str {
    "a2x-client stub"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        assert_eq!(stub(), "a2x-client stub");
    }
}
