// a2x-agents — Built-in agent implementations
// See plans/05-agents.md
//
// Stub crate — to be implemented in Phase 1.

pub fn stub() -> &'static str {
    "a2x-agents stub"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        assert_eq!(stub(), "a2x-agents stub");
    }
}
