module 0x1::BorrowField {

    struct T1 has key, drop { y: u64 }
    spec T1 {
        invariant y > 10;
    }

    struct T2 has key, drop { x: u64 }

    fun borrow_field_from_S_no_specs_incorrect(t2: T2): u64 {
        t2.x
    }

    fun borrow_field_from_P_no_specs(): u64 {
        let t2 = T2 { x: 0 };
        t2.x
    }

    fun borrow_field_from_P_with_specs_incorrect(): u64 {
        let t1 = T1 { y: 0 };
        t1.y
    }

    fun borrow_field_from_S_with_specs_incorrect(t1: T1): u64 {
        t1.y
    }
}
