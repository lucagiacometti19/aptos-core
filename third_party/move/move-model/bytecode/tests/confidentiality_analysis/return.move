module 0x1::Return {

    // x is secret because it's an arg - implicit flow
    fun return_P_while_pc_S_incorrect(x: u64): u64 {
        if (x > 0) {
            // pc raised
            return 1
        };
        0
    }

    // x is secret because it's an arg - implicit flow
    fun return_S_while_pc_S_incorrect(x: u64): u64 {
        if (x > 0) {
            // pc raised
            return x
        };
        0
    }

    // x is secret because it's an arg - explicit flow
    fun return_S_while_pc_P_incorrect(x: u64): u64 {
        x
    }

    // should not complain
    fun return_P_while_pc_P(): u64 {
        0
    }
}
