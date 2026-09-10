#![cfg(feature = "alloy")]

use alloy_primitives::Bytes;
use evm_asm::{alloy_bytecode, bytecode};

#[test]
fn wraps_the_same_static_bytecode_as_the_raw_macro() {
    const CODE: Bytes = alloy_bytecode! { push1 0x08 calldatasize eq };
    const RAW: &[u8; 4] = bytecode! { push1 0x08 calldatasize eq };
    static EMPTY: Bytes = alloy_bytecode! {};

    assert_eq!(CODE.as_ref(), RAW);
    assert_eq!(CODE, Bytes::from_static(RAW));
    assert!(EMPTY.is_empty());
}

#[test]
fn supports_256_bit_operands_and_amsterdam_instructions() {
    let code = alloy_bytecode! {
        push32 0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff
        slotnum
        dupn 17
        swapn 108
        exchange 2 3
    };
    let raw = bytecode! {
        push32 0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff
        slotnum
        dupn 17
        swapn 108
        exchange 2 3
    };
    assert_eq!(code.as_ref(), raw);
}

#[test]
fn resolves_bytes_without_a_local_bytes_import() {
    mod caller {
        pub const CODE: alloy_primitives::Bytes = evm_asm::alloy_bytecode! { push0 stop };
    }
    assert_eq!(caller::CODE.as_ref(), &[0x5f, 0x00]);
}
