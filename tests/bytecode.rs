use evm_asm::bytecode;

#[test]
fn assembles_the_requested_example() {
    let code: &[u8] = bytecode! {
        push1 0x08
        calldatasize
        eq
        push1 0x0a
        jumpi
        push0
        push0
        revert

        jumpdest
        push0
        calldataload
        push1 0xc0
        shr
        timestamp
        gt
        push1 0x16
        jumpi
        stop

        jumpdest
        push0
        push0
        revert
    };

    assert_eq!(
        code,
        b"\x60\x08\x36\x14\x60\x0a\x57\x5f\x5f\xfd\x5b\x5f\x35\x60\xc0\x1c\x42\x11\x60\x16\x57\x00\x5b\x5f\x5f\xfd"
    );
    assert_eq!(code[0x0a], 0x5b);
    assert_eq!(code[0x16], 0x5b);
}

#[test]
fn supports_constants_empty_input_comments_and_separators() {
    const EMPTY: &[u8; 0] = bytecode! {};
    const CODE: &[u8] = bytecode! {
        PUSH2 0xAB_CD, // A comma may separate instructions.
        Push0;
        /* Rust keywords are valid opcode names. */
        mod return
    };
    assert!(EMPTY.is_empty());
    assert_eq!(CODE, &[0x61, 0xab, 0xcd, 0x5f, 0x06, 0xf3]);
}

#[test]
fn returns_a_static_array_reference_that_coerces_to_a_slice() {
    static CODE: &[u8; 2] = bytecode! { push0 stop };
    const OWNED: [u8; 2] = *bytecode! { push0 stop };
    fn slice() -> &'static [u8] {
        bytecode! { push0 stop }
    }

    assert_eq!(CODE, &[0x5f, 0x00]);
    assert_eq!(OWNED, *CODE);
    assert_eq!(slice(), CODE);
}

#[test]
fn encodes_integer_bases_and_left_padding() {
    let code = bytecode! {
        push1 255
        push1 0b1010_0101
        push2 0o777
        push4 0x123
        push2 0
        push1 0x0001
    };
    assert_eq!(
        code,
        b"\x60\xff\x60\xa5\x61\x01\xff\x63\x00\x00\x01\x23\x61\x00\x00\x60\x01"
    );
}

#[test]
fn supports_full_256_bit_literals() {
    let hex = bytecode! {
        push32 0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff
    };
    let decimal = bytecode! {
        push32 115792089237316195423570985008687907853269984665640564039457584007913129639935
    };
    let address = bytecode! { push20 0x1234567890abcdef1234567890abcdef12345678 };
    assert_eq!(hex.len(), 33);
    assert_eq!(hex[0], 0x7f);
    assert_eq!(hex[1..], [0xff; 32]);
    assert_eq!(decimal, hex);
    assert_eq!(
        address,
        b"\x73\x12\x34\x56\x78\x90\xab\xcd\xef\x12\x34\x56\x78\x90\xab\xcd\xef\x12\x34\x56\x78"
    );
}

#[test]
fn encodes_arithmetic_comparison_and_bitwise_opcodes() {
    let code = bytecode! {
        stop add mul sub div sdiv mod smod addmod mulmod exp signextend
        lt gt slt sgt eq iszero and or xor not byte shl shr sar clz keccak256
    };
    assert_eq!(
        code,
        b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0a\x0b\x10\x11\x12\x13\x14\x15\x16\x17\x18\x19\x1a\x1b\x1c\x1d\x1e\x20"
    );
}

#[test]
fn encodes_environment_and_block_opcodes() {
    let code = bytecode! {
        address balance origin caller callvalue calldataload calldatasize calldatacopy
        codesize codecopy gasprice extcodesize extcodecopy returndatasize returndatacopy
        extcodehash blockhash coinbase timestamp number prevrandao gaslimit chainid
        selfbalance basefee blobhash blobbasefee slotnum
    };
    assert_eq!(code.as_slice(), &(0x30..=0x4b).collect::<Vec<u8>>());
}

#[test]
fn encodes_memory_storage_and_control_flow_opcodes() {
    let code = bytecode! {
        pop mload mstore mstore8 sload sstore jump jumpi pc msize gas jumpdest
        tload tstore mcopy push0
    };
    assert_eq!(code.as_slice(), &(0x50..=0x5f).collect::<Vec<u8>>());
}

#[test]
fn encodes_dup_swap_log_and_system_opcodes() {
    let code = bytecode! {
        dup1 dup2 dup3 dup4 dup5 dup6 dup7 dup8 dup9 dup10 dup11 dup12 dup13 dup14 dup15 dup16
        swap1 swap2 swap3 swap4 swap5 swap6 swap7 swap8 swap9 swap10 swap11 swap12 swap13
        swap14 swap15 swap16 log0 log1 log2 log3 log4
        create call callcode return delegatecall create2 staticcall revert invalid selfdestruct
    };
    let mut expected = (0x80..=0xa4).collect::<Vec<u8>>();
    expected.extend_from_slice(&[0xf0, 0xf1, 0xf2, 0xf3, 0xf4, 0xf5, 0xfa, 0xfd, 0xfe, 0xff]);
    assert_eq!(code.as_slice(), expected);
}

#[test]
fn accepts_common_opcode_aliases() {
    assert_eq!(
        bytecode! { sha3 keccak difficulty suicide },
        bytecode! { keccak256 keccak256 prevrandao selfdestruct }
    );
}

#[test]
fn encodes_amsterdam_stack_instructions_using_eip_8024_vectors() {
    const CODE: &[u8] = bytecode! {
        SLOTNUM
        DUPN 17
        swapn 108
        dupn 0x11
        jumpdest
        exchange 2 3
        exchange 1 19
        exchange 14 16
        exchange 14 15
        stop
    };
    assert_eq!(
        CODE,
        b"\x4b\xe6\x80\xe7\xdb\xe6\x80\x5b\xe8\x9d\xe8\x2f\xe8\x50\xe8\x51\x00"
    );
}

#[test]
fn encodes_amsterdam_stack_operand_boundaries() {
    let code = bytecode! {
        dupn 17 dupn 144 dupn 145 dupn 235
        swapn 17 swapn 144 swapn 145 swapn 235
        exchange 1 2 exchange 1 16 exchange 1 17 exchange 1 29
    };
    assert_eq!(
        code,
        b"\xe6\x80\xe6\xff\xe6\x00\xe6\x5a\xe7\x80\xe7\xff\xe7\x00\xe7\x5a\xe8\x8e\xe8\x80\xe8\x4f\xe8\x8f"
    );
}
