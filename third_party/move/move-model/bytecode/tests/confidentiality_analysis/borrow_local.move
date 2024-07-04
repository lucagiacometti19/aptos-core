module 0x1::BorrowLocal {

    fun dummy(_: &u64) { }

    fun borrow_local_P() {
        let x = 0;
        let ref_x = &x;
        dummy(ref_x)
    }

    fun borrow_local_S_incorrect(x: u64) {
        let ref_x = &x;
        dummy(ref_x)
    }
}
