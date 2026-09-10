#![no_std]

use evm_asm::bytecode;

const ROOT_WORD: [u8; 2] = [0x12, 0x34];

mod constants {
    pub const ARRAY: [u8; 2] = [0xab, 0xcd];
    pub const REFERENCE: &[u8; 2] = &ARRAY;
    pub const SLICE: &[u8] = &[0xef, 0x01];
}

#[test]
fn interpolates_arrays_slices_and_module_paths_between_literals() {
    use constants::ARRAY;
    const CODE: &[u8; 13] = bytecode! {
        push0
        push2 #ARRAY,
        push1 0xff;
        push2 #constants::REFERENCE
        push2 #constants::SLICE
        stop
    };
    assert_eq!(
        CODE,
        b"\x5f\x61\xab\xcd\x60\xff\x61\xab\xcd\x61\xef\x01\x00"
    );
}

#[test]
fn accepts_const_accessors_blocks_and_nested_macros() {
    struct Word([u8; 2]);
    impl Word {
        const fn as_bytes(&self) -> &[u8] {
            &self.0
        }
    }
    const WORD: Word = Word([0x11, 0x22]);
    static CODE: &[u8; 12] = bytecode! {
        push2 #(WORD.as_bytes())
        push2 #({ let mut bytes = [0x33, 0]; bytes[1] = 0x44; bytes })
        push2 #(const { [0x55, 0x66] })
        push2 #(bytecode! { push0 stop })
    };
    assert_eq!(CODE, b"\x61\x11\x22\x61\x33\x44\x61\x55\x66\x61\x5f\x00");
}

#[test]
fn accepts_associated_and_qualified_constants() {
    struct Config;
    impl Config {
        const WORD: [u8; 2] = [0xab, 0xcd];
    }
    trait Words {
        const WORD: &'static [u8];
    }
    impl Words for Config {
        const WORD: &'static [u8] = &[0xef, 0x01];
    }
    let code = bytecode! {
        push2 #Config::WORD
        push2 #<Config as Words>::WORD
        push2 #crate::ROOT_WORD
    };
    assert_eq!(code, b"\x61\xab\xcd\x61\xef\x01\x61\x12\x34");
}

#[test]
fn returns_static_data_from_functions_and_supports_const_parameters() {
    fn single<const BYTE: u8>() -> &'static [u8; 2] {
        bytecode! { push1 #([BYTE]) }
    }
    const WORD: [u8; 32] = *b"0123456789abcdefghijklmnopqrstuv";
    fn full_word() -> &'static [u8; 33] {
        bytecode! { push32 #WORD }
    }
    const OWNED: [u8; 3] = *bytecode! { push2 #ROOT_WORD };

    assert_eq!(single::<0>(), &[0x60, 0]);
    assert_eq!(single::<255>(), &[0x60, 255]);
    assert_eq!(full_word()[0], 0x7f);
    assert_eq!(&full_word()[1..], &WORD);
    assert_eq!(OWNED, [0x61, 0x12, 0x34]);
}

#[test]
#[allow(non_upper_case_globals)]
fn caller_constants_do_not_conflict_with_expansion() {
    const __evm_asm_output: [u8; 1] = [0x11];
    const __evm_asm_source: [u8; 1] = [0x22];
    const __evm_asm_index: [u8; 1] = [0x33];
    let code = bytecode! {
        push1 #__evm_asm_output
        push1 #__evm_asm_source
        push1 #__evm_asm_index
    };
    assert_eq!(code, b"\x60\x11\x60\x22\x60\x33");
}

#[test]
fn interpolations_preserve_instruction_and_jump_offsets() {
    const DESTINATION: [u8; 1] = [0x06];
    const VALUE: &[u8] = &[0xab, 0xcd];
    let code = bytecode! {
        push1 #DESTINATION
        jump
        push2 #VALUE
        jumpdest
        slotnum
        dupn 17
        swapn 108
        exchange 2 3
    };
    assert_eq!(
        code,
        b"\x60\x06\x56\x61\xab\xcd\x5b\x4b\xe6\x80\xe7\xdb\xe8\x9d"
    );
    assert_eq!(code[usize::from(DESTINATION[0])], 0x5b);
}

#[test]
fn numeric_interpolation_fits_every_push_width() {
    const SIZE: u64 = 7;
    macro_rules! check {
        ($($push:ident),* $(,)?) => {$(
            assert_eq!(
                bytecode! { $push #~(SIZE.to_be_bytes()) },
                bytecode! { $push 7 },
            );
            assert_eq!(
                bytecode! { $push #~left(SIZE.to_be_bytes()) },
                bytecode! { $push 7 },
            );
            let right = bytecode! { $push #~right(SIZE.to_le_bytes()) };
            let left = bytecode! { $push 7 };
            assert_eq!(right.len(), left.len());
            assert_eq!(right[0], left[0]);
            assert_eq!(right[1], 7);
            assert!(right[2..].iter().all(|&byte| byte == 0));
        )*};
    }
    check!(
        push1, push2, push3, push4, push5, push6, push7, push8, push9, push10, push11, push12,
        push13, push14, push15, push16, push17, push18, push19, push20, push21, push22, push23,
        push24, push25, push26, push27, push28, push29, push30, push31, push32,
    );
}

#[test]
fn numeric_interpolation_preserves_big_endian_order_and_mixes_with_exact_operands() {
    const WORD: &[u8] = &[0, 0, 0x12, 0x34];
    const CODE: &[u8; 20] = bytecode! {
        push1 0xff
        push2 #~WORD,
        push4 #~constants::REFERENCE;
        push3 #~constants::SLICE
        push2 #ROOT_WORD
        push2 #~constants::ARRAY
    };
    assert_eq!(
        CODE,
        bytecode! {
            push1 0xff push2 0x1234 push4 0xabcd
            push3 0xef01 push2 0x1234 push2 0xabcd
        },
    );
}

#[test]
fn numeric_interpolation_accepts_empty_and_oversized_zero_inputs() {
    const EMPTY_ARRAY: [u8; 0] = [];
    const EMPTY_SLICE: &[u8] = &[];
    const ZEROS: &[u8; 40] = &[0; 40];
    assert_eq!(
        bytecode! {
            push1 #~EMPTY_ARRAY
            push32 #~EMPTY_SLICE
            push1 #~ZEROS
            push32 #~ZEROS
            push1 #~right(EMPTY_ARRAY)
            push32 #~right(EMPTY_SLICE)
            push1 #~right(ZEROS)
            push32 #~right(ZEROS)
        },
        bytecode! {
            push1 0 push32 0 push1 0 push32 0
            push1 0 push32 0 push1 0 push32 0
        },
    );
}

#[test]
fn numeric_interpolation_keeps_all_significant_bytes_at_the_boundary() {
    const WORD: [u8; 34] = {
        let mut bytes = [0xff; 34];
        bytes[0] = 0;
        bytes[1] = 0;
        bytes
    };
    assert_eq!(
        bytecode! { push1 #~(255u64.to_be_bytes()) push2 #~(256u64.to_be_bytes()) },
        bytecode! { push1 255 push2 256 },
    );
    let code = bytecode! { push32 #~WORD };
    assert_eq!(code[0], 0x7f);
    assert_eq!(&code[1..], &[0xff; 32]);
    const RIGHT_WORD: [u8; 34] = {
        let mut bytes = [0xff; 34];
        bytes[32] = 0;
        bytes[33] = 0;
        bytes
    };
    assert_eq!(bytecode! { push32 #~right(RIGHT_WORD) }, code);
    assert_eq!(
        bytecode! { push1 #~right(255u64.to_le_bytes()) },
        bytecode! { push1 255 },
    );
}

#[test]
fn numeric_interpolation_supports_const_accessors_and_parameters() {
    struct Word(u64);
    impl Word {
        const fn as_bytes(&self) -> [u8; 8] {
            self.0.to_be_bytes()
        }
    }
    const WORD: Word = Word(0x1234);
    static CODE: &[u8; 5] = bytecode! { push4 #~(WORD.as_bytes()) };
    fn single<const VALUE: u64>() -> &'static [u8; 2] {
        bytecode! { push1 #~(VALUE.to_be_bytes()) }
    }
    assert_eq!(CODE, bytecode! { push4 0x1234 });
    assert_eq!(single::<0>(), bytecode! { push1 0 });
    assert_eq!(single::<255>(), bytecode! { push1 255 });
}

#[test]
#[allow(non_upper_case_globals)]
fn helper_names_do_not_conflict_with_caller_constants() {
    const source: [u8; 1] = [1];
    const index: [u8; 1] = [2];
    const start: [u8; 1] = [3];
    const count: [u8; 1] = [4];
    const output: [u8; 1] = [5];
    const operands: [u8; 1] = [6];
    const __evm_asm: [u8; 1] = [7];
    const padding: [u8; 1] = [8];
    const end: [u8; 1] = [9];
    assert_eq!(
        bytecode! {
            push1 #source
            push2 #~index
            push1 #start
            push2 #~count
            push1 #output
            push2 #~operands
            push1 #~__evm_asm
            push2 #~right(padding)
            push2 #~left(end)
        },
        bytecode! {
            push1 1 push2 2 push1 3 push2 4 push1 5 push2 6 push1 7
            push2 0x0800 push2 9
        },
    );
}

#[test]
fn explicit_padding_sides_preserve_byte_order_and_instruction_offsets() {
    const LEADING: [u8; 4] = [0, 0, 0x12, 0x34];
    const TRAILING: &[u8] = &[0x56, 0x78, 0, 0];
    static CODE: &[u8; 27] = bytecode! {
        push4 #~left(constants::ARRAY),
        push4 #~right(constants::REFERENCE);
        push2 #~right(TRAILING)
        push2 #~left(LEADING)
        push2 #constants::SLICE
        push2 #~right(constants::ARRAY)
        push3 #~right(bytecode! { push0 stop })
        jumpdest
    };
    assert_eq!(
        CODE,
        bytecode! {
            push4 0xabcd push4 0xabcd0000 push2 0x5678 push2 0x1234
            push2 0xef01 push2 0xabcd push3 0x5f0000 jumpdest
        },
    );
    assert_eq!(CODE[26], 0x5b);
}

#[test]
#[allow(non_upper_case_globals)]
fn padding_mode_names_remain_valid_constant_paths() {
    const left: [u8; 2] = [0x12, 0x34];
    const right: &[u8] = &[0x56, 0x78];
    mod left {
        pub const WORD: [u8; 2] = [0x9a, 0xbc];
    }
    assert_eq!(
        bytecode! {
            push3 #~left
            push3 #~right
            push3 #~left::WORD
            push3 #~left(right)
            push3 #~right(left)
        },
        bytecode! {
            push3 0x1234 push3 0x5678 push3 0x9abc push3 0x5678 push3 0x123400
        },
    );
}

#[test]
fn explicit_padding_supports_const_functions_blocks_and_parameters() {
    const fn right() -> [u8; 2] {
        [0x12, 0x34]
    }
    fn single<const VALUE: u64>() -> &'static [u8; 2] {
        bytecode! { push1 #~right(VALUE.to_le_bytes()) }
    }
    const CODE: &[u8] = bytecode! {
        push4 #~(right())
        push4 #~left(right())
        push4 #~right({ let bytes = right(); [bytes[1], bytes[0]] })
    };
    assert_eq!(
        CODE,
        bytecode! { push4 0x1234 push4 0x1234 push4 0x34120000 }
    );
    assert_eq!(single::<0>(), bytecode! { push1 0 });
    assert_eq!(single::<255>(), bytecode! { push1 255 });
}
