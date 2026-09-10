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
