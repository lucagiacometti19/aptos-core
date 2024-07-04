module 0x1::ReadRef {

    fun read_ref_S_while_pc_P_incorrect(x: &u64): u64 {
        *x
    }

    fun read_ref_S_while_pc_S_incorrect(x: u64, y: &u64): u64 {
        if (x > 0) {
            return *y
        };
        // will be flagged since this return is part of the else branch
        0
    }

    fun read_ref_P_while_pc_S_incorrect(x: u64): u64 {
        let y = &0;
        if (x > 0) {
            return *y
        };
        // will be flagged since this return is part of the else branch
        0
    }

    fun read_ref_P_while_pc_P(): u64 {
        let x = &0;
        *x
    }
}
