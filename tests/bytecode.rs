use evm_asm::bytecode;
use hex_literal::hex;

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
    assert_eq!(CODE, &hex!("61abcd5f06f3"));
}

#[test]
fn returns_a_static_array_reference_that_coerces_to_a_slice() {
    static CODE: &[u8; 2] = bytecode! { push0 stop };
    const OWNED: [u8; 2] = *bytecode! { push0 stop };
    fn slice() -> &'static [u8] {
        bytecode! { push0 stop }
    }

    assert_eq!(CODE, &hex!("5f00"));
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
    assert_eq!(code, &hex!("60ff60a56101ff63000001236100006001"));
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
    assert_eq!(address, &hex!("731234567890abcdef1234567890abcdef12345678"));
}

#[test]
fn encodes_arithmetic_comparison_and_bitwise_opcodes() {
    let code = bytecode! {
        stop add mul sub div sdiv mod smod addmod mulmod exp signextend
        lt gt slt sgt eq iszero and or xor not byte shl shr sar clz keccak256
    };
    assert_eq!(
        code,
        &hex!("000102030405060708090a0b101112131415161718191a1b1c1d1e20")
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
    expected.extend_from_slice(&hex!("f0f1f2f3f4f5fafdfeff"));
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
    assert_eq!(CODE, &hex!("4be680e7dbe6805be89de82fe850e85100"));
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
        &hex!("e680e6ffe600e65ae780e7ffe700e75ae88ee880e84fe88f")
    );
}

#[test]
fn assembles_expiry_verifier() {
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
        &hex!("60083614600a575f5ffd5b5f3560c01c4211601657005b5f5ffd")
    );
    assert_eq!(code[0x0a], 0x5b);
    assert_eq!(code[0x16], 0x5b);
}

#[test]
fn assembles_recent_root() {
    let code: &[u8] = bytecode! {
        callvalue
        push2 0x00ba
        jumpi

        calldatasize
        push1 0x40
        eq
        push2 0x00c0
        jumpi

        calldatasize
        push1 0x48
        calldatasize
        mod
        push2 0x00ba
        jumpi

        dup1
        iszero
        push2 0x00ba
        jumpi

        push2 0x0480
        dup2
        gt
        push2 0x00ba
        jumpi

        slotnum
        push1 0x00

        jumpdest
        push1 0x20
        dup2
        add
        calldataload
        push1 0xc0
        shr
        dup3
        dup2
        lt
        iszero
        push2 0x00ba
        jumpi
        dup1
        dup4
        sub
        push2 0x2000
        gt
        iszero
        push2 0x00ba
        jumpi
        push32 0x8f42481679c8e6fefa040974b3c905e0ce3f2e464ba93acdb074a41181617efc
        push1 0x00
        mstore
        push1 0x48
        dup3
        push1 0x20
        calldatacopy
        push1 0x68
        push1 0x00
        keccak256
        push32 0xbdc897da2177d260ff5f4be5d4b2aad43f89c3347a305b584fa5a2546d053daa
        push1 0x00
        mstore
        swap1
        push2 0x1fff
        and
        push1 0xc0
        shl
        push1 0x40
        mstore
        push1 0x48
        push1 0x00
        keccak256
        sload
        eq
        iszero
        push2 0x00ba
        jumpi
        push1 0x48
        add
        dup3
        dup2
        lt
        push2 0x0028
        jumpi
        stop

        jumpdest
        push1 0x00
        push1 0x00
        revert

        jumpdest
        caller
        push1 0x00
        mstore
        push1 0x20
        push1 0x00
        push1 0x20
        calldatacopy
        push1 0x34
        push1 0x0c
        keccak256
        dup1
        push32 0x8f42481679c8e6fefa040974b3c905e0ce3f2e464ba93acdb074a41181617efc
        push1 0x40
        mstore
        slotnum
        push1 0x68
        mstore
        push1 0x60
        mstore
        push1 0x20
        push1 0x20
        push1 0x88
        calldatacopy
        push1 0x68
        push1 0x40
        keccak256
        dup2
        push32 0xbdc897da2177d260ff5f4be5d4b2aad43f89c3347a305b584fa5a2546d053daa
        push1 0xa8
        mstore
        push2 0x1fff
        slotnum
        and
        push1 0xd0
        mstore
        push1 0xc8
        mstore
        push1 0x48
        push1 0xa8
        keccak256
        sstore
        stop
    };

    assert_eq!(
        code,
        &hex!(
            "346100ba57366040146100c05736604836066100ba5780156100ba5761048081116100ba574b60005b602081013560c01c828110156100ba5780830361200011156100ba577f8f42481679c8e6fefa040974b3c905e0ce3f2e464ba93acdb074a41181617efc60005260488260203760686000207fbdc897da2177d260ff5f4be5d4b2aad43f89c3347a305b584fa5a2546d053daa60005290611fff1660c01b60405260486000205414156100ba5760480182811061002857005b60006000fd5b33600052602060006020376034600c20807f8f42481679c8e6fefa040974b3c905e0ce3f2e464ba93acdb074a41181617efc6040524b606852606052602060206088376068604020817fbdc897da2177d260ff5f4be5d4b2aad43f89c3347a305b584fa5a2546d053daa60a852611fff4b1660d05260c852604860a8205500"
        )
    );
    assert_eq!(code[0x28], 0x5b);
    assert_eq!(code[0xba], 0x5b);
    assert_eq!(code[0xc0], 0x5b);
}
