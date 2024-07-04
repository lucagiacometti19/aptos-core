module 0x1::Unpack {

    struct T1 {
        f1: u64
    }

    fun pack_single_S_into_T1_incorrect(x: u64): T1 {
        // pack x produces S
        let t1 = T1 { f1: x };
        return t1
    }

    fun pack_single_P_into_T1(): T1 {
        let x = 0;
        // pack x produces P
        let t1 = T1 { f1: x };
        return t1
    }

    struct T2 {
        f1: u64,
        f2: bool
    }

    fun pack_one_S_into_T2_incorrect(x: u64): T2 {
        let y = false;
        // pack of x (S) and y (P) produces S
        let t2 = T2 { f1: x, f2: y };
        return t2
    }

    fun pack_two_S_into_T2_incorrect(x: u64, y: bool): T2 {
        // pack of x (S) and y (S) produces S
        let t2 = T2 { f1: x, f2: y };
        return t2
    }

    fun pack_two_P_into_T2(): T2 {
        let x = 0;
        let y = true;
        // pack of x (P) and y (P) produces P
        let t2 = T2 { f1: x, f2: y };
        return t2
    }
}
