module 0x1::BorrowGlobal {

    struct T1 has key, copy { y: u64 }
    spec T1 {
        invariant y > 10;
    }

    struct T2 has key, copy { x: u64 }

    // At bytecode level, this test:
    // 1 - borrows the global at address addr
    // 2 - reads the ref, allowed by the copy ability

    fun borrow_global_no_specs_with_P_address(): T2 acquires T2 {
        let addr: address = @0x19;
        // t2 is P
        let t2 = borrow_global<T2>(addr);
        return *t2
    }

    fun borrow_global_no_specs_with_S_address_incorrect(addr: address): T2 acquires T2 {
        // t2 is S
        let t2 = borrow_global<T2>(addr);
        return *t2
    }

    fun borrow_global_with_specs_with_S_address_incorrect(addr: address): T1 acquires T1 {
        // T1 has specs -> t1 is S
        let t1 = borrow_global<T1>(addr);
        return *t1
    }

    fun borrow_global_with_specs_with_P_address_incorrect(): T1 acquires T1 {
        let addr: address = @0x19;
        // T1 has specs -> t1 is S
        let t1 = borrow_global<T1>(addr);
        return *t1
    }
}
