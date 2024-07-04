module 0x1::Call {

    fun dummy(_: u64): u64 {
        0
    }

    //------------------------------------------------------------------
    // Call to User Defined Fn (UDFn)

    // Call to UDFn with public arg while pc is secret
    // x is secret because it's an arg - implicit flow
    fun UDFn_call_P_args_while_pc_S_incorrect(x: u64) {
        if (x > 0) {
            // pc raised
            dummy(0);
        };
    }

    // call to user defined fn (UDFn) with secret arg while pc is secret
    // x is secret because it's an arg - implicit flow
    fun UDFn_call_S_args_while_pc_S_incorrect(x: u64) {
        if (x > 0) {
            // pc raised
            dummy(x);
        };
    }

    // call to user defined fn (UDFn) with public arg while pc is public
    // should not complain
    fun UDFn_call_P_args_while_pc_P() {
        dummy(0);
    }

    // call to user defined fn (UDFn) with secret arg while pc is public
    // x is secret because it's an arg - explicit flow
    fun UDFn_call_S_args_while_pc_P_incorrect(x: u64) {
        dummy(x);
    }
}
