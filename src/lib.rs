//! Assemble EVM bytecode at compile time with [`bytecode!`].

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use ruint::aliases::U256;
use syn::{
    Error, Ident, LitByteStr, LitInt, Result, Token,
    ext::IdentExt,
    parse::{Parse, ParseStream},
};

mod opcodes;

/// Assembles opcodes into a byte-string literal of type `&'static [u8; N]`.
///
/// The result coerces to `&'static [u8]` and works in `const` and `static`
/// initializers. Assembly happens at compile time with no runtime allocation.
/// This macro is always available and does not require Alloy. Enable the
/// `alloy` feature to also use `alloy_bytecode!`, which returns Alloy's `Bytes`.
///
/// Opcode names are case-insensitive and may be separated by whitespace,
/// commas, or semicolons. Rust comments are allowed.
///
/// `push1` through `push32` each take one unsigned, unsuffixed integer literal
/// in hexadecimal, decimal, binary, or octal notation. Values are encoded in
/// big-endian order, padded on the left with zeros to the requested width.
/// `push0` takes no operand. Rust expressions and variables are not supported.
///
/// Amsterdam adds `slotnum` (no operands), `dupn n`, `swapn n`, and
/// `exchange n m`. Stack operands are logical indices, encoded by the macro
/// according to [EIP-8024](https://eips.ethereum.org/EIPS/eip-8024):
/// `dupn`/`swapn` require `17 <= n <= 235`; `exchange` requires
/// `1 <= n < m` and `n + m <= 30`. Use `dup1`–`dup16` and `swap1`–`swap16`
/// for shallower accesses. Like PUSH operands, indices must be unsigned,
/// unsuffixed integer literals.
///
/// ```
/// use evm_asm::bytecode;
/// let code = bytecode! { slotnum dupn 17 swapn 108 exchange 2 3 };
/// assert_eq!(code, &[0x4b, 0xe6, 0x80, 0xe7, 0xdb, 0xe8, 0x9d]);
/// ```
///
/// ```
/// use evm_asm::bytecode;
///
/// const CODE: &[u8] = bytecode! {
///     push2 0x1234
///     push0
///     mstore
///     push1 0x20
///     push0
///     return
/// };
/// assert_eq!(CODE, &[0x61, 0x12, 0x34, 0x5f, 0x52, 0x60, 0x20, 0x5f, 0xf3]);
/// ```
///
/// Unknown opcodes, missing operands, and overflowing immediates are compile
/// errors:
///
/// ```compile_fail
/// use evm_asm::bytecode;
/// let code = bytecode! { unknown_opcode };
/// ```
///
/// ```compile_fail
/// use evm_asm::bytecode;
/// let code = bytecode! { push1 };
/// ```
///
/// ```compile_fail
/// use evm_asm::bytecode;
/// let code = bytecode! { push1 0x100 };
/// ```
///
/// This is an assembler: it does not validate stack usage, jump destinations,
/// or opcode availability on a particular EVM fork.
#[proc_macro]
pub fn bytecode(input: TokenStream) -> TokenStream {
    assemble(input)
        .map(|bytes| quote!(#bytes))
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

/// Assembles opcodes into an `alloy_primitives::Bytes` backed by static data.
///
/// Requires the `alloy` feature and an `alloy-primitives` dependency in the
/// consuming crate. Uses the same syntax, validation, and assembly as
/// [`bytecode!`], wrapping the byte-string literal in `Bytes::from_static`.
/// No runtime parsing, allocation, or byte copying is needed.
///
/// ```
/// use alloy_primitives::Bytes;
/// use evm_asm::{alloy_bytecode, bytecode};
///
/// const CODE: Bytes = alloy_bytecode! { push1 0x08 calldatasize eq };
/// const RAW: &[u8] = bytecode! { push1 0x08 calldatasize eq };
/// assert_eq!(CODE.as_ref(), RAW);
/// ```
///
/// Invalid instructions and operands produce the same compile errors as
/// [`bytecode!`]:
///
/// ```compile_fail
/// use evm_asm::alloy_bytecode;
/// let code = alloy_bytecode! { unknown_opcode };
/// ```
///
/// ```compile_fail
/// use evm_asm::alloy_bytecode;
/// let code = alloy_bytecode! { push1 };
/// ```
///
/// ```compile_fail
/// use evm_asm::alloy_bytecode;
/// let code = alloy_bytecode! { push1 0x100 };
/// ```
#[cfg(feature = "alloy")]
#[proc_macro]
pub fn alloy_bytecode(input: TokenStream) -> TokenStream {
    assemble(input)
        .map(|bytes| quote!(::alloy_primitives::Bytes::from_static(#bytes)))
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

fn assemble(input: TokenStream) -> Result<LitByteStr> {
    let Bytecode(bytes) = syn::parse(input)?;
    Ok(LitByteStr::new(&bytes, Span::call_site()))
}

#[derive(Debug)]
struct Bytecode(Vec<u8>);

impl Parse for Bytecode {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut bytes = Vec::new();

        while !input.is_empty() {
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
                continue;
            }
            if input.peek(Token![;]) {
                input.parse::<Token![;]>()?;
                continue;
            }

            // EVM names such as `return` and `mod` are Rust keywords.
            let name = input
                .call(Ident::parse_any)
                .map_err(|_| input.error("expected an opcode name"))?;
            let mnemonic = name.to_string().to_ascii_lowercase();
            let opcode = opcodes::lookup(&mnemonic)
                .ok_or_else(|| Error::new(name.span(), format!("unknown opcode `{name}`")))?;
            bytes.push(opcode);

            match opcode {
                0x60..=0x7f => {
                    let width = usize::from(opcode - 0x5f);
                    let immediate = parse_operand(input, &mnemonic)?;
                    bytes.extend_from_slice(&encode_immediate(&immediate, width)?);
                }
                0xe6 | 0xe7 => {
                    let n = parse_stack_index(input, &mnemonic, 17, 235)?;
                    // EIP-8024 encode_single: (n + 111) mod 256.
                    bytes.push(n.wrapping_add(111));
                }
                0xe8 => {
                    let n = parse_stack_index(input, &mnemonic, 1, 14)?;
                    let m = parse_stack_index(input, &mnemonic, n + 1, 30 - n)?;
                    // EIP-8024 encode_pair avoids reserved immediate bytes.
                    let (q, r) = if m <= 16 {
                        (n - 1, m - 1)
                    } else {
                        (29 - m, n - 1)
                    };
                    bytes.push((16 * q + r) ^ 143);
                }
                _ => {}
            }
        }

        Ok(Self(bytes))
    }
}

fn parse_operand(input: ParseStream<'_>, mnemonic: &str) -> Result<LitInt> {
    if input.peek(Token![-]) || !input.peek(LitInt) {
        return Err(input.error(format!(
            "`{mnemonic}` requires an unsigned integer literal operand"
        )));
    }
    let literal: LitInt = input.parse()?;
    if !literal.suffix().is_empty() {
        return Err(Error::new(
            literal.span(),
            format!("`{mnemonic}` operands must be unsuffixed integer literals"),
        ));
    }
    Ok(literal)
}

fn parse_stack_index(input: ParseStream<'_>, mnemonic: &str, min: u8, max: u8) -> Result<u8> {
    let literal = parse_operand(input, mnemonic)?;
    let index = literal.base10_parse::<u8>().ok();
    match index {
        Some(index) if (min..=max).contains(&index) => Ok(index),
        _ => Err(Error::new(
            literal.span(),
            format!("`{mnemonic}` stack index must be in {min}..={max}"),
        )),
    }
}

fn encode_immediate(literal: &LitInt, width: usize) -> Result<Vec<u8>> {
    let overflow = || {
        Error::new(
            literal.span(),
            format!("operand does not fit in `push{width}` ({width} bytes)"),
        )
    };
    // syn normalizes all Rust integer bases without imposing a u128 limit.
    let value = U256::from_str_radix(literal.base10_digits(), 10).map_err(|_| overflow())?;
    let encoded = value.to_be_bytes::<32>();
    let start = encoded.len() - width;
    if encoded[..start].iter().any(|&byte| byte != 0) {
        return Err(overflow());
    }
    Ok(encoded[start..].to_vec())
}

#[cfg(test)]
mod tests {
    use super::Bytecode;

    #[test]
    fn every_push_width_pads_and_checks_its_boundary() {
        for width in 1..=32 {
            let small = syn::parse_str::<Bytecode>(&format!("push{width} 1")).unwrap();
            let mut expected = vec![0; width + 1];
            expected[0] = 0x5f + width as u8;
            expected[width] = 1;
            assert_eq!(small.0, expected);

            let maximum = format!("push{width} 0x{}", "ff".repeat(width));
            let maximum = syn::parse_str::<Bytecode>(&maximum).unwrap();
            assert_eq!(maximum.0[0], 0x5f + width as u8);
            assert_eq!(maximum.0[1..], vec![0xff; width]);

            let too_large = format!("push{width} 0x1{}", "00".repeat(width));
            let error = syn::parse_str::<Bytecode>(&too_large).unwrap_err();
            assert_eq!(
                error.to_string(),
                format!("operand does not fit in `push{width}` ({width} bytes)")
            );
        }
    }

    #[test]
    fn invalid_input_has_useful_errors() {
        for (source, message) in [
            ("push33 0", "unknown opcode `push33`"),
            ("dup0", "unknown opcode `dup0`"),
            ("dup17", "unknown opcode `dup17`"),
            ("swap0", "unknown opcode `swap0`"),
            ("swap17", "unknown opcode `swap17`"),
            ("log5", "unknown opcode `log5`"),
            ("push01 0", "unknown opcode `push01`"),
            ("push", "unknown opcode `push`"),
            ("push1", "requires an unsigned integer literal operand"),
            ("push1 stop", "requires an unsigned integer literal operand"),
            ("push1 -1", "requires an unsigned integer literal operand"),
            ("push1 1.5", "requires an unsigned integer literal operand"),
            (
                "push1 VALUE",
                "requires an unsigned integer literal operand",
            ),
            (
                "push1 \"ff\"",
                "requires an unsigned integer literal operand",
            ),
            ("push1 1u8", "must be unsuffixed integer literals"),
            ("push1 1f32", "must be unsuffixed integer literals"),
            ("push0 0", "expected an opcode name"),
            ("add 1", "expected an opcode name"),
            ("push1 1 + 2", "expected an opcode name"),
        ] {
            let error = syn::parse_str::<Bytecode>(source).unwrap_err();
            assert!(
                error.to_string().contains(message),
                "{source}: expected {message:?}, got {error}"
            );
        }
    }

    #[test]
    fn amsterdam_stack_operands_are_validated() {
        for mnemonic in ["dupn", "swapn"] {
            for operand in [
                "0",
                "16",
                "236",
                "256",
                "0xffffffffffffffffffffffffffffffff",
            ] {
                let source = format!("{mnemonic} {operand}");
                let error = syn::parse_str::<Bytecode>(&source).unwrap_err();
                assert_eq!(
                    error.to_string(),
                    format!("`{mnemonic}` stack index must be in 17..=235")
                );
            }
        }

        for source in [
            "exchange 0 1",
            "exchange 1 1",
            "exchange 3 2",
            "exchange 1 30",
            "exchange 14 17",
            "exchange 15 16",
            "exchange 1 256",
        ] {
            let error = syn::parse_str::<Bytecode>(source).unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("`exchange` stack index must be in")
            );
        }

        for instruction in ["dupn", "swapn", "exchange", "exchange 2"] {
            for operand in ["", "stop", "-1", "1.5", "\"17\"", "VALUE"] {
                let source = format!("{instruction} {operand}");
                let error = syn::parse_str::<Bytecode>(&source).unwrap_err();
                assert!(
                    error
                        .to_string()
                        .contains("requires an unsigned integer literal operand")
                );
            }
            let source = format!("{instruction} 17u8");
            let error = syn::parse_str::<Bytecode>(&source).unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("must be unsuffixed integer literals")
            );
        }
        assert!(syn::parse_str::<Bytecode>("slotnum 1").is_err());
    }

    #[test]
    fn amsterdam_stack_encoding_round_trips_every_valid_operand() {
        for mnemonic in ["dupn", "swapn"] {
            let mut seen = [false; 256];
            for n in 17..=235 {
                let code = syn::parse_str::<Bytecode>(&format!("{mnemonic} {n}")).unwrap();
                assert_eq!(code.0.len(), 2);
                let immediate = usize::from(code.0[1]);
                assert!(!seen[immediate]);
                seen[immediate] = true;
                // EIP-8024 decode_single.
                assert_eq!((immediate + 145) % 256, n);
            }
            for (immediate, used) in seen.into_iter().enumerate() {
                assert_eq!(used, immediate <= 90 || immediate >= 128);
            }
        }

        let mut seen = [false; 256];
        for n in 1..=14 {
            for m in n + 1..=30 - n {
                let code = syn::parse_str::<Bytecode>(&format!("exchange {n} {m}")).unwrap();
                assert_eq!(code.0.len(), 2);
                let immediate = usize::from(code.0[1]);
                assert!(!seen[immediate]);
                seen[immediate] = true;
                // EIP-8024 decode_pair, independent of the encoder's branches.
                let k = immediate ^ 143;
                let (q, r) = (k / 16, k % 16);
                let decoded = if q < r {
                    (q + 1, r + 1)
                } else {
                    (r + 1, 29 - q)
                };
                assert_eq!(decoded, (n, m));
            }
        }
        for (immediate, used) in seen.into_iter().enumerate() {
            assert_eq!(used, immediate <= 81 || immediate >= 128);
        }
    }
}
