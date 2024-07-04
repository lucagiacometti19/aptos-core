module 0x1::MoveFrom {

    struct T1 has key, drop { y: u64 }
    spec T1 {
        invariant y > 10;
    }

    struct T2 has key, drop { x: u64 }

    // Struct T2 has no specs
    fun move_from_no_specs_while_pc_S_incorrect(addr: address, x: u64): T2 acquires T2 {
        // p is P
        let p = T2 { x: 0 };
        if (x > 0) {
            // T2 has no specs, but pc is S -> s is S
            p = move_from<T2>(addr);
            return p
        };
        // will be flagged since this return is part of the else branch
        p
    }

    fun move_from_no_specs_while_pc_P(addr: address): T2 acquires T2 {
        // p is P
        // address_of will be flagged as implicit data leak via call
        let p = move_from<T2>(addr);
        return p
    }

    // Struct T1 has specs
    fun move_from_with_specs_while_pc_P_incorrect(addr: address): T1 acquires T1 {
        // s is S because of specs
        // address_of will be flagged as implicit data leak via call
        let s = move_from<T1>(addr);
        return s
    }

    fun move_from_with_specs_while_pc_S_incorrect(addr: address, x: u64): T1 acquires T1 {
        // s is P
        let s = T1 { y: 0 };
        if (x > 0) {
            // T1 has specs + pc is S -> s is S
            // address_of will be flagged as implicit data leak via call
            s = move_from<T1>(addr);
            return s
        };
        // will be flagged since this return is part of the else branch
        s
    }
}
