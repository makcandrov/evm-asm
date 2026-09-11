//! Optional semantic references for opcode hovers; not compiled without opcode-docs.

use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;

pub(super) fn expand(opcodes: &[(Ident, u8)], bytecode: TokenStream) -> TokenStream {
    if opcodes.is_empty() {
        return bytecode;
    }
    let mut names: [Option<Ident>; 256] = core::array::from_fn(|_| None);
    let mut definitions = Vec::new();
    let mut references = Vec::new();
    for (input, opcode) in opcodes {
        let declaration = names[usize::from(*opcode)].get_or_insert_with(|| {
            let info = info(*opcode).expect("supported opcode must have hover documentation");
            let name = Ident::new(&info.name, Span::call_site());
            let docs = info.docs;
            definitions.push(quote! {
                #[doc = #docs]
                pub const #name: u8 = #opcode;
            });
            name
        });
        // Only this reference gets the input span. Declarations and helper
        // tokens must not create additional semantic targets for the opcode.
        let mut reference = declaration.clone();
        reference.set_span(input.span());
        references.push(quote! { let _ = __evm_asm_opcode_docs::#reference; });
    }
    quote! {{
        const {
            mod __evm_asm_opcode_docs {
                #(#definitions)*
            }
            #(#references)*
        };
        // Caller operands remain outside the helper module's scope.
        #bytecode
    }}
}

struct Info {
    name: String,
    docs: String,
}

// Descriptions and stack effects follow the Amsterdam execution specifications.
// Sources are linked directly from each generated hover.
fn make_info(
    opcode: u8,
    name: String,
    summary: String,
    stack: String,
    operands: String,
    section: &str,
) -> Info {
    let source = match opcode {
        0x1e => "https://eips.ethereum.org/EIPS/eip-7939".to_string(),
        0x4b => "https://eips.ethereum.org/EIPS/eip-7843".to_string(),
        0xe6..=0xe8 => "https://eips.ethereum.org/EIPS/eip-8024".to_string(),
        _ => format!(
            "https://github.com/ethereum/execution-specs/blob/forks/amsterdam/src/ethereum/forks/amsterdam/vm/instructions/{section}.py"
        ),
    };
    let docs = format!(
        "# {name} (0x{opcode:02x})\n\n{summary}\n\n**Stack:** {stack}\n\n**Assembly operands:** {operands}\n\n[Execution specification]({source})"
    );
    Info { name, docs }
}

fn info(opcode: u8) -> Option<Info> {
    match opcode {
        0x60..=0x7f => {
            let width = opcode - 0x5f;
            return Some(make_info(
                opcode,
                format!("PUSH{width}"),
                format!("Push a {width}-byte big-endian immediate as a 256-bit stack word."),
                "Pops 0, pushes 1.".to_string(),
                format!(
                    "`push{width} value`: an integer literal or `#...` / `#~...` byte interpolation."
                ),
                "stack",
            ));
        }
        0x80..=0x8f => {
            let depth = opcode - 0x7f;
            return Some(make_info(
                opcode,
                format!("DUP{depth}"),
                format!("Copy stack item {depth} onto the top; positions start at 1."),
                format!("Requires {depth} items; adds one copy without removing the original."),
                "None.".to_string(),
                "stack",
            ));
        }
        0x90..=0x9f => {
            let depth = opcode - 0x8f;
            return Some(make_info(
                opcode,
                format!("SWAP{depth}"),
                format!(
                    "Exchange the top stack item with item {}; positions start at 1.",
                    depth + 1
                ),
                format!("Requires {} items; stack height is unchanged.", depth + 1),
                "None.".to_string(),
                "stack",
            ));
        }
        0xa0..=0xa4 => {
            let topics = opcode - 0xa0;
            return Some(make_info(
                opcode,
                format!("LOG{topics}"),
                format!(
                    "Emit a log from memory with {topics} topics; consume offset, size, then topics."
                ),
                format!("Pops {}, pushes 0.", 2 + topics),
                "None.".to_string(),
                "log",
            ));
        }
        _ => {}
    }
    let (name, pops, pushes, summary, section) = match opcode {
        0x00 => (
            "STOP",
            0,
            0,
            "Halt successfully with empty return data.",
            "control_flow",
        ),
        0x01 => ("ADD", 2, 1, "Add two words modulo 2^256.", "arithmetic"),
        0x02 => (
            "MUL",
            2,
            1,
            "Multiply two words modulo 2^256.",
            "arithmetic",
        ),
        0x03 => (
            "SUB",
            2,
            1,
            "Subtract the second popped word from the first, modulo 2^256.",
            "arithmetic",
        ),
        0x04 => (
            "DIV",
            2,
            1,
            "Unsigned division; a zero divisor yields zero.",
            "arithmetic",
        ),
        0x05 => (
            "SDIV",
            2,
            1,
            "Signed division, rounded toward zero; a zero divisor yields zero.",
            "arithmetic",
        ),
        0x06 => (
            "MOD",
            2,
            1,
            "Unsigned remainder; a zero divisor yields zero.",
            "arithmetic",
        ),
        0x07 => (
            "SMOD",
            2,
            1,
            "Signed remainder with the dividend's sign; a zero divisor yields zero.",
            "arithmetic",
        ),
        0x08 => (
            "ADDMOD",
            3,
            1,
            "Compute (a + b) modulo m without intermediate overflow; m = 0 yields zero.",
            "arithmetic",
        ),
        0x09 => (
            "MULMOD",
            3,
            1,
            "Compute (a * b) modulo m without intermediate overflow; m = 0 yields zero.",
            "arithmetic",
        ),
        0x0a => (
            "EXP",
            2,
            1,
            "Raise the first popped word to the second, modulo 2^256.",
            "arithmetic",
        ),
        0x0b => (
            "SIGNEXTEND",
            2,
            1,
            "Extend the sign of byte index b in a word, counting from the least significant byte.",
            "arithmetic",
        ),
        0x10 => (
            "LT",
            2,
            1,
            "Push 1 if the first popped word is less than the second (unsigned), otherwise 0.",
            "comparison",
        ),
        0x11 => (
            "GT",
            2,
            1,
            "Push 1 if the first popped word is greater than the second (unsigned), otherwise 0.",
            "comparison",
        ),
        0x12 => (
            "SLT",
            2,
            1,
            "Signed less-than comparison, producing 1 or 0.",
            "comparison",
        ),
        0x13 => (
            "SGT",
            2,
            1,
            "Signed greater-than comparison, producing 1 or 0.",
            "comparison",
        ),
        0x14 => (
            "EQ",
            2,
            1,
            "Compare two words for equality, producing 1 or 0.",
            "comparison",
        ),
        0x15 => (
            "ISZERO",
            1,
            1,
            "Push 1 for a zero word, otherwise 0.",
            "comparison",
        ),
        0x16 => ("AND", 2, 1, "Bitwise conjunction of two words.", "bitwise"),
        0x17 => (
            "OR",
            2,
            1,
            "Bitwise inclusive disjunction of two words.",
            "bitwise",
        ),
        0x18 => (
            "XOR",
            2,
            1,
            "Bitwise exclusive disjunction of two words.",
            "bitwise",
        ),
        0x19 => ("NOT", 1, 1, "Invert every bit of a word.", "bitwise"),
        0x1a => (
            "BYTE",
            2,
            1,
            "Read a byte by zero-based index from the most significant end; indices >= 32 yield zero.",
            "bitwise",
        ),
        0x1b => (
            "SHL",
            2,
            1,
            "Shift a word left by the popped bit count, discarding overflow.",
            "bitwise",
        ),
        0x1c => (
            "SHR",
            2,
            1,
            "Shift a word right by the popped bit count, inserting zeros.",
            "bitwise",
        ),
        0x1d => (
            "SAR",
            2,
            1,
            "Shift a signed word right by the popped bit count, extending its sign.",
            "bitwise",
        ),
        0x1e => (
            "CLZ",
            1,
            1,
            "Count leading zero bits in a 256-bit word; zero yields 256. EIP-7939.",
            "bitwise",
        ),
        0x20 => (
            "KECCAK256",
            2,
            1,
            "Hash memory[offset..offset + size] with Keccak-256. Aliases: keccak, sha3.",
            "keccak",
        ),
        0x30 => (
            "ADDRESS",
            0,
            1,
            "Push the address of the current execution context.",
            "environment",
        ),
        0x31 => (
            "BALANCE",
            1,
            1,
            "Read an account's balance in wei.",
            "environment",
        ),
        0x32 => (
            "ORIGIN",
            0,
            1,
            "Push the transaction sender's address.",
            "environment",
        ),
        0x33 => (
            "CALLER",
            0,
            1,
            "Push the immediate caller's address.",
            "environment",
        ),
        0x34 => (
            "CALLVALUE",
            0,
            1,
            "Push this call's transferred value in wei.",
            "environment",
        ),
        0x35 => (
            "CALLDATALOAD",
            1,
            1,
            "Read 32 bytes of calldata at an offset, zero-filling beyond its end.",
            "environment",
        ),
        0x36 => (
            "CALLDATASIZE",
            0,
            1,
            "Push the calldata length in bytes.",
            "environment",
        ),
        0x37 => (
            "CALLDATACOPY",
            3,
            0,
            "Copy calldata into memory; consume destination, source offset, and size.",
            "environment",
        ),
        0x38 => (
            "CODESIZE",
            0,
            1,
            "Push the executing code's length in bytes.",
            "environment",
        ),
        0x39 => (
            "CODECOPY",
            3,
            0,
            "Copy executing code into memory; consume destination, source offset, and size.",
            "environment",
        ),
        0x3a => (
            "GASPRICE",
            0,
            1,
            "Push the transaction's effective gas price.",
            "environment",
        ),
        0x3b => (
            "EXTCODESIZE",
            1,
            1,
            "Read an account's code length.",
            "environment",
        ),
        0x3c => (
            "EXTCODECOPY",
            4,
            0,
            "Copy account code into memory; consume address, destination, source offset, and size.",
            "environment",
        ),
        0x3d => (
            "RETURNDATASIZE",
            0,
            1,
            "Push the last child call's return-data length.",
            "environment",
        ),
        0x3e => (
            "RETURNDATACOPY",
            3,
            0,
            "Copy return data into memory; out-of-bounds source ranges fail.",
            "environment",
        ),
        0x3f => (
            "EXTCODEHASH",
            1,
            1,
            "Read an account's code hash; a nonexistent or empty account yields zero.",
            "environment",
        ),
        0x40 => (
            "BLOCKHASH",
            1,
            1,
            "Read a block hash from the previous 256 blocks; an unavailable block yields zero.",
            "block",
        ),
        0x41 => (
            "COINBASE",
            0,
            1,
            "Push the current block's fee-recipient address.",
            "block",
        ),
        0x42 => (
            "TIMESTAMP",
            0,
            1,
            "Push the current block's Unix timestamp in seconds.",
            "block",
        ),
        0x43 => ("NUMBER", 0, 1, "Push the current block number.", "block"),
        0x44 => (
            "PREVRANDAO",
            0,
            1,
            "Push the block's beacon-chain randomness value. Alias: difficulty.",
            "block",
        ),
        0x45 => (
            "GASLIMIT",
            0,
            1,
            "Push the current block's gas limit.",
            "block",
        ),
        0x46 => ("CHAINID", 0, 1, "Push the chain identifier.", "block"),
        0x47 => (
            "SELFBALANCE",
            0,
            1,
            "Push the executing account's balance.",
            "environment",
        ),
        0x48 => (
            "BASEFEE",
            0,
            1,
            "Push the block's base fee per gas.",
            "environment",
        ),
        0x49 => (
            "BLOBHASH",
            1,
            1,
            "Read a transaction blob's versioned hash by index; an invalid index yields zero.",
            "environment",
        ),
        0x4a => (
            "BLOBBASEFEE",
            0,
            1,
            "Push the block's base fee per blob gas.",
            "environment",
        ),
        0x4b => (
            "SLOTNUM",
            0,
            1,
            "Push the consensus-layer slot number. Amsterdam / EIP-7843.",
            "block",
        ),
        0x50 => ("POP", 1, 0, "Discard the top stack word.", "stack"),
        0x51 => (
            "MLOAD",
            1,
            1,
            "Read a 32-byte word from memory at an offset.",
            "memory",
        ),
        0x52 => (
            "MSTORE",
            2,
            0,
            "Write a 32-byte word to memory; consume offset, then value.",
            "memory",
        ),
        0x53 => (
            "MSTORE8",
            2,
            0,
            "Write the least significant byte of a word to memory.",
            "memory",
        ),
        0x54 => (
            "SLOAD",
            1,
            1,
            "Read a word from persistent storage by key.",
            "storage",
        ),
        0x55 => (
            "SSTORE",
            2,
            0,
            "Write persistent storage; consume key, then value.",
            "storage",
        ),
        0x56 => (
            "JUMP",
            1,
            0,
            "Jump to a byte offset that contains a valid JUMPDEST.",
            "control_flow",
        ),
        0x57 => (
            "JUMPI",
            2,
            0,
            "Consume destination and condition; jump when the condition is nonzero.",
            "control_flow",
        ),
        0x58 => (
            "PC",
            0,
            1,
            "Push the byte offset of this instruction.",
            "control_flow",
        ),
        0x59 => (
            "MSIZE",
            0,
            1,
            "Push the active memory size in bytes.",
            "memory",
        ),
        0x5a => (
            "GAS",
            0,
            1,
            "Push the gas remaining after charging this instruction.",
            "control_flow",
        ),
        0x5b => (
            "JUMPDEST",
            0,
            0,
            "Mark a valid jump destination.",
            "control_flow",
        ),
        0x5c => ("TLOAD", 1, 1, "Read transient storage by key.", "storage"),
        0x5d => (
            "TSTORE",
            2,
            0,
            "Write transient storage; its contents last only for this transaction.",
            "storage",
        ),
        0x5e => (
            "MCOPY",
            3,
            0,
            "Copy memory with overlap support; consume destination, source, and size.",
            "memory",
        ),
        0x5f => (
            "PUSH0",
            0,
            1,
            "Push zero onto the stack without an immediate operand.",
            "stack",
        ),
        0xe6 => (
            "DUPN",
            0,
            1,
            "Duplicate stack item n onto the top (positions start at 1). Requires n items. Amsterdam / EIP-8024.",
            "stack",
        ),
        0xe7 => (
            "SWAPN",
            0,
            0,
            "Exchange the top stack item with item n + 1 (positions start at 1). Amsterdam / EIP-8024.",
            "stack",
        ),
        0xe8 => (
            "EXCHANGE",
            0,
            0,
            "Exchange stack items n + 1 and m + 1 (positions start at 1). Amsterdam / EIP-8024.",
            "stack",
        ),
        0xf0 => (
            "CREATE",
            3,
            1,
            "Create a contract from value and a memory region of initcode; return its address or zero on failure.",
            "system",
        ),
        0xf1 => (
            "CALL",
            7,
            1,
            "Call another account with gas, value, and input/output memory ranges; return a success flag.",
            "system",
        ),
        0xf2 => (
            "CALLCODE",
            7,
            1,
            "Execute another account's code in the current account's context; return a success flag.",
            "system",
        ),
        0xf3 => (
            "RETURN",
            2,
            0,
            "Halt successfully and return a memory region; consume offset, then size.",
            "system",
        ),
        0xf4 => (
            "DELEGATECALL",
            6,
            1,
            "Execute other code in the current context, retaining caller and value; return a success flag.",
            "system",
        ),
        0xf5 => (
            "CREATE2",
            4,
            1,
            "Create a contract with a salt-derived address; consume value, offset, size, and salt.",
            "system",
        ),
        0xfa => (
            "STATICCALL",
            6,
            1,
            "Call another account with state modifications forbidden; return a success flag.",
            "system",
        ),
        0xfd => (
            "REVERT",
            2,
            0,
            "Revert this call's state changes and return a memory region; preserve unused gas.",
            "system",
        ),
        0xfe => (
            "INVALID",
            0,
            0,
            "Halt with an invalid-instruction exception.",
            "__init__",
        ),
        0xff => (
            "SELFDESTRUCT",
            1,
            0,
            "Transfer the account's balance to a beneficiary and halt; deletion is restricted by EIP-6780. Alias: suicide.",
            "system",
        ),
        _ => return None,
    };
    let operands = match opcode {
        0xe6 | 0xe7 => "`n`: an unsigned, unsuffixed literal in 17..=235; encoded automatically.",
        0xe8 => {
            "`n m`: unsigned, unsuffixed literals with 1 <= n < m and n + m <= 30; encoded automatically."
        }
        _ => "None.",
    };
    Some(make_info(
        opcode,
        name.to_string(),
        summary.to_string(),
        format!("Pops {pops}, pushes {pushes}."),
        operands.to_string(),
        section,
    ))
}

#[cfg(test)]
mod tests {
    use super::info;
    use crate::{Bytecode, opcodes};
    use alloc::{format, string::ToString, vec, vec::Vec};
    use proc_macro2::{TokenStream, TokenTree};

    #[test]
    fn documentation_covers_the_supported_instruction_set() {
        for opcode in 0..=u8::MAX {
            let supported = matches!(
                opcode,
                0x00..=0x0b | 0x10..=0x1e | 0x20 | 0x30..=0x4b |
                0x50..=0xa4 | 0xe6..=0xe8 | 0xf0..=0xf5 | 0xfa | 0xfd..=0xff
            );
            let entry = info(opcode);
            assert_eq!(entry.is_some(), supported, "opcode 0x{opcode:02x}");
            if let Some(entry) = entry {
                assert_eq!(
                    opcodes::lookup(&entry.name.to_ascii_lowercase()),
                    Some(opcode)
                );
                assert!(entry.docs.contains(&format!("0x{opcode:02x}")));
                assert!(entry.docs.contains("**Stack:**"));
                assert!(entry.docs.contains("**Assembly operands:**"));
                assert!(entry.docs.contains("[Execution specification](https://"));
                let operand = match opcode {
                    0x60..=0x7f => " 0",
                    0xe6 | 0xe7 => " 17",
                    0xe8 => " 2 3",
                    _ => "",
                };
                let code = syn::parse_str::<Bytecode>(&format!("{}{operand}", entry.name)).unwrap();
                assert!(!code.expand().is_empty());
            }
        }
        assert!(info(0xe7).unwrap().docs.contains("item n + 1"));
        assert!(info(0xe8).unwrap().docs.contains("items n + 1 and m + 1"));
    }

    #[test]
    fn aliases_share_definitions_and_each_opcode_has_one_spanned_reference() {
        fn leaves(tokens: TokenStream) -> Vec<TokenTree> {
            tokens
                .into_iter()
                .flat_map(|token| match token {
                    TokenTree::Group(group) => leaves(group.stream()),
                    token => vec![token],
                })
                .collect()
        }
        let source = "push1 7 PUSH1 7 sha3 keccak keccak256 return mod";
        let code = syn::parse_str::<Bytecode>(source).unwrap();
        let expanded = code.expand();
        let text = expanded.to_string();
        assert_eq!(text.matches("pub const ").count(), 4);
        assert_eq!(text.matches("pub const PUSH1 ").count(), 1);
        assert_eq!(text.matches("pub const KECCAK256 ").count(), 1);
        let tokens = leaves(expanded);
        for (input, opcode) in &code.opcode_refs {
            let matches: Vec<_> = tokens
                .iter()
                .filter(|token| token.span().byte_range() == input.span().byte_range())
                .map(ToString::to_string)
                .collect();
            assert_eq!(matches, vec![info(*opcode).unwrap().name]);
        }
    }
}
