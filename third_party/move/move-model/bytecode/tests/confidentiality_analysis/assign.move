module 0x1::Assign {

    fun assign_P_while_pc_P(): u64 {
        // load const, x is P
        let x = 0;
        // pc is P, x is P -> y is P
        let y = x;
        y
    }

    fun assign_P_while_pc_S_incorrect(s: u64): u64 {
        // load const, x is P
        let x = 1;
        // load const, y is P
        let y = 0;
        if (s > 0) {
            // pc is S, x is P -> y is S
            y = x;
        };
        y
    }

    fun assign_S_while_pc_S_incorrect(x: u64, s: u64): u64 {
        // load const, y is P
        let y = 0;
        if (s > 0) {
            // pc is S, x is S -> y is S
            y = x;
        };
        y
    }

    fun assign_S_while_pc_P_incorrect(x: u64): u64 {
        // pc is P, x is S -> y is S
        let y = x;
        y
    }
}
