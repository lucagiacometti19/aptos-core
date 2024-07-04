module 0x1::LoadConst {

    fun load_const_while_pc_S_incorrect(x: u64): u64 {
        if (x > 0) {
            // pc raised, v is S
            let v = 0;
            return v
        };
        0
    }

    fun load_const_while_pc_P(_: u64): u64 {
        let v = 0;
        return v
    }
}
