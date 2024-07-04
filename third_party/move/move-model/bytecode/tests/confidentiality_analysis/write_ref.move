module 0x1::WriteRef {

    fun write_ref_P_while_pc_P() {
        let x = &mut 0;
        // pc is P -> const 1 is P -> allowed
        *x = 1;
    }

    fun write_ref_P_while_pc_S(y: u64) {
        let c = 1;
        let x = &mut 0;
        if (y > 0) {
            // pc is S -> const c is P -> not allowed
            *x = c;
        };
        // needed to prevent bytecode optimization that would prevent
        // the test to be (correctly) flagged
        c = c + 1;
    }

    fun write_ref_S_while_pc_S(y: u64) {
        let x = &mut 0;
        if (y > 0) {
            // pc is S -> y is S -> allowed
            *x = y;
        };
    }

    fun write_ref_S_while_pc_P(y: u64) {
        let x = &mut 0;
        // pc is P -> y is S -> allowed
        *x = y;
    }
}
