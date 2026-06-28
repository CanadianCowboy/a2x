// a2x-gateway — Entity gateway, protocol listeners, auth
// See plans/06-entity-gateway.md
//
// Stub crate — to be implemented in Phase 6.

pub fn stub() -> &'static str {
    "a2x-gateway stub"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        assert_eq!(stub(), "a2x-gateway stub");
    }
}
