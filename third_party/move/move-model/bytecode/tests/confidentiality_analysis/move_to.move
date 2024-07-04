module 0x1::MoveTo {

    struct T1 has key, drop { y: u64 }
    spec T1 {
        invariant y > 10;
    }

    struct T2 has key, drop { x: u64 }

    // Struct T2 has no specs
    fun move_to_S_no_specs_while_pc_S(account: &signer, s: T2, x: u64) {
        if (x > 0) {
            move_to(account, s)
        };
    }

    fun move_to_S_no_specs_while_pc_P_incorrect(account: &signer, s: T2) {
        move_to(account, s)
    }

    fun move_to_P_no_specs_while_pc_S(account: &signer, x: u64) {
        // p is P
        let p = T2 { x: 0 };
        if (x > 0) {
            move_to(account, p)
        };
    }

    fun move_to_P_no_specs_while_pc_P(account: &signer) {
        // p is P
        let p = T2 { x: 0 };
        move_to(account, p)
    }

    // Struct T1 has specs
    fun move_to_S_with_specs_while_pc_P_incorrect(account: &signer, s: T1) {
        move_to(account, s)
    }

    fun move_to_P_with_specs_while_pc_P_incorrect(account: &signer) {
        // p is P but has specs
        let p = T1 { y: 0 };
        move_to(account, p)
    }

    fun move_to_P_with_specs_while_pc_S(account: &signer, x: u64) {
        // p is P but has specs
        let p = T1 { y: 0 };
        if (x > 0) {
            move_to(account, p)
        };
    }

    fun move_to_S_with_specs_while_pc_S(account: &signer, s: T1, x: u64) {
        if (x > 0) {
            move_to(account, s)
        };
    }

}
