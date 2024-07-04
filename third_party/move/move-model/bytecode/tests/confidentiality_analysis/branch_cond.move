module 0x1::BranchCond {

    fun branch_guard_S_incorrect(x: u64): u64 {
        if (x > 0) {
            // pc raised
            return 1
        };
        0
    }

    fun branch_guard_P(_: u64): u64 {
        let p = 1;
        if (p > 0) {
            // pc isn't raised
            return 1
        };
        0
    }

    fun branch_guard_S_while_pc_S_incorrect(x: u64, y: u64): u64 {
        if (x > 0) {
            // pc raised
            if (y > 0) {
                // pc still S
                return 1
            };
            return 0
        };
        0
    }

    /* TODO #1: public var can't be declared inside fn. p > 0 is True already in the bytecode since its value is known
            at compile time.
    fun branch_guard_P_while_pc_S_incorrect(x: u64): u64 {
        let p = 1;
        if (x > 0) {
            // pc raised
            if (p > 0) {
                // pc still S
                return 1
            };
            return 5
        };
        0
    }
    */
}
