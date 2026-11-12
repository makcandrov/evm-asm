#![cfg(feature = "opcode-docs")]
#![no_std]
#![deny(warnings)]

use evm_asm::bytecode;
use hex_literal::hex;

mod __evm_asm_opcode_docs {
    pub const VALUE: [u8; 1] = [0x22];
}

#[test]
fn documentation_keeps_const_output_and_caller_names_intact() {
    const PUSH1: [u8; 1] = [0x11];
    const CODE: &[u8; 8] = bytecode! {
        push1 #PUSH1
        PUSH1 #__evm_asm_opcode_docs::VALUE
        mod
        return
        sha3
        keccak256
    };
    assert_eq!(CODE, &hex!("6011602206f32020"));
    const EMPTY: &[u8; 0] = bytecode! {};
    assert!(EMPTY.is_empty());
}
