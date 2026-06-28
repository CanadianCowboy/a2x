// a2x-bus — Message bus, routing, transport
// See plans/04-bus.md
//
// Stub crate — to be implemented in Phase 0.

pub fn stub() -> &'static str {
    "a2x-bus stub"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        assert_eq!(stub(), "a2x-bus stub");
    }
}
