// a2x-ccs — CCS cognitive runtime VM
// See plans/03-ccs-vm.md
//
// Stub crate — to be implemented in Phase 0/2.

pub fn stub() -> &'static str {
    "a2x-ccs stub"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        assert_eq!(stub(), "a2x-ccs stub");
    }
}
