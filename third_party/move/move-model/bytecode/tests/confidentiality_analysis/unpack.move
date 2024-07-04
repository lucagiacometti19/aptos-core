module 0x1::Unpack {

    struct T {
        f1: u64,
        f2: bool
    }

    fun unpack_struct_S_incorrect(s: T): (u64, bool) {
        // unpack of s produces S
        let T { f1: x, f2: y } = s;
        return (x, y)
    }

    fun unpack_struct_P(): (u64, bool) {
        let p = T { f1: 0, f2: false };
        // unpack of p produces P
        let T { f1: x, f2: y } = p;
        return (x, y)
    }
}
