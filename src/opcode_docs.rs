//! Optional semantic references for opcode hovers; not compiled without opcode-docs.

use alloc::{
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use core::fmt::Write;
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

// Stack presentation is inspired by evm.codes. Descriptions are maintained here;
// execution specifications and gas schedules are linked from each hover.
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
        0xfe => "https://eips.ethereum.org/EIPS/eip-141".to_string(),
        _ => format!(
            "https://github.com/ethereum/execution-specs/blob/master/src/ethereum/forks/osaka/vm/instructions/{section}.py"
        ),
    };
    let since = since(opcode);
    let (minimum, note) = minimum_gas(opcode);
    let draft = matches!(opcode, 0x4b | 0xe6..=0xe8);
    let schedule = if draft { "Amsterdam draft" } else { "Osaka" };
    let gas = match minimum {
        Some(minimum) => format!("{minimum} ({schedule})."),
        None => "Not applicable: this opcode always fails.".to_string(),
    };
    let note = if note.is_empty() {
        String::new()
    } else {
        format!(" {note}")
    };
    let diagram = stack_diagram(opcode);
    let stack_note = if matches!(opcode, 0x80..=0x9f | 0xe6..=0xe8) {
        " Stack positions start at 1 (`s1` is the top)."
    } else {
        ""
    };
    let links = if draft {
        format!("[Execution specification]({source})")
    } else {
        format!(
            "[evm.codes](https://www.evm.codes/#{opcode:02x}) · [Execution specification]({source}) · [Gas schedule](https://github.com/ethereum/execution-specs/blob/master/src/ethereum/forks/osaka/vm/gas.py)"
        )
    };
    let docs = format!(
        "# {name} (0x{opcode:02x})\n\n{summary}\n\n**Since:** {since}\n\n**Minimum gas:** {gas}{note}\n\n**Stack:** {stack}{stack_note}\n\n{diagram}\n\n**Assembly operands:** {operands}\n\n{links}"
    );
    Info { name, docs }
}

// Fork names refer to opcode availability, not to the gas schedule below.
// Osaka costs come from execution-specs' osaka/vm/gas.py and instruction bodies.
// Amsterdam additions use EIP-7843 and EIP-8024; their activation is still draft.
fn since(opcode: u8) -> &'static str {
    match opcode {
        0xf4 => "Homestead",
        0x3d | 0x3e | 0xfa | 0xfd => "Byzantium",
        0x1b..=0x1d | 0x3f | 0xf5 => "Constantinople",
        0x46 | 0x47 => "Istanbul",
        0x48 => "London",
        0x44 => "Paris (0x44 was DIFFICULTY since Frontier)",
        0x5f => "Shanghai",
        0x49 | 0x4a | 0x5c..=0x5e => "Cancun",
        0x1e => "Osaka",
        0x4b | 0xe6..=0xe8 => "Amsterdam (draft)",
        0xfe => "Frontier (invalid byte; designated INVALID by EIP-141)",
        _ => "Frontier",
    }
}

// Minimum opcode charge before refunds, assuming warm access and no additional
// memory expansion where applicable. None means an unconditional exceptional halt.
fn minimum_gas(opcode: u8) -> (Option<u32>, &'static str) {
    let minimum = match opcode {
        0x00 | 0xf3 | 0xfd => 0,
        0x5b => 1,
        0x30
        | 0x32..=0x34
        | 0x36
        | 0x38
        | 0x3a
        | 0x3d
        | 0x41..=0x46
        | 0x48
        | 0x4a
        | 0x4b
        | 0x50
        | 0x58..=0x5a
        | 0x5f => 2,
        0x01
        | 0x03
        | 0x10..=0x1d
        | 0x35
        | 0x37
        | 0x39
        | 0x3e
        | 0x49
        | 0x51..=0x53
        | 0x5e
        | 0x60..=0x9f
        | 0xe6..=0xe8 => 3,
        0x02 | 0x04..=0x07 | 0x0b | 0x1e | 0x47 => 5,
        0x08 | 0x09 | 0x56 => 8,
        0x0a | 0x57 => 10,
        0x40 => 20,
        0x20 => 30,
        0x31 | 0x3b | 0x3c | 0x3f | 0x54 | 0x55 | 0x5c | 0x5d | 0xf1 | 0xf2 | 0xf4 | 0xfa => 100,
        0xa0..=0xa4 => 375 * (1 + u32::from(opcode - 0xa0)),
        0xf0 | 0xf5 => 32_000,
        0xfe => {
            return (
                None,
                "All remaining gas in this call is consumed by the exceptional halt.",
            );
        }
        0xff => 5_000,
        _ => unreachable!("unsupported opcode"),
    };
    let note = match opcode {
        0x0a => "Add 50 gas per significant exponent byte; the minimum uses exponent zero.",
        0x20 => "Add 6 gas per 32-byte word hashed, rounded up, plus memory expansion.",
        0x31 | 0x3b | 0x3f => "Warm account access; a cold account costs 2,600 gas.",
        0x37 | 0x39 | 0x3e | 0x5e => {
            "Add 3 gas per 32-byte word copied, rounded up, plus memory expansion."
        }
        0x3c => {
            "Warm account access; a cold account costs 2,600 gas. Add 3 gas per 32-byte word copied, rounded up, plus memory expansion."
        }
        0x51..=0x53 | 0xf3 | 0xfd => "Memory expansion can add gas.",
        0x54 => "Warm storage read; a cold slot costs 2,100 gas.",
        0x55 => {
            "Minimum for a warm no-op or dirty-slot write, before refunds. Cold access and value changes can cost more. Requires more than 2,300 gas remaining even for a 100-gas write."
        }
        0xa0..=0xa4 => "Includes the topic charge. Add 8 gas per data byte, plus memory expansion.",
        0xf0 => {
            "Add initcode metering, memory expansion, initcode execution, and deployed-code storage costs."
        }
        0xf5 => {
            "Add initcode hashing and metering, memory expansion, initcode execution, and deployed-code storage costs."
        }
        0xf1 | 0xf2 => {
            "Minimum for a warm target and zero value. Cold or delegated-code access, value transfer, memory expansion, and child execution can add gas; CALL may also create an account."
        }
        0xf4 | 0xfa => {
            "Minimum for a warm target. Cold or delegated-code access, memory expansion, and child execution can add gas."
        }
        0xff => "Cold beneficiary access and creating a beneficiary account can add gas.",
        _ => "",
    };
    (Some(minimum), note)
}

// Lists are in pop order: index zero is the top of the stack. Keep this data
// independent of the drawing orientation so noncommutative operands stay clear.
fn stack_io(opcode: u8) -> (Vec<String>, Vec<String>) {
    if matches!(opcode, 0x80..=0x9f) {
        let depth = if opcode < 0x90 {
            opcode - 0x7f
        } else {
            opcode - 0x8f + 1
        };
        let input: Vec<_> = (1..=depth).map(|n| format!("s{n}")).collect();
        let mut output = input.clone();
        if opcode < 0x90 {
            output.insert(0, input[usize::from(depth - 1)].clone());
        } else {
            output.swap(0, usize::from(depth - 1));
        }
        return (input, output);
    }
    if matches!(opcode, 0xa0..=0xa4) {
        let mut input = vec!["offset".to_string(), "size".to_string()];
        input.extend((1..=opcode - 0xa0).map(|n| format!("topic{n}")));
        return (input, Vec::new());
    }
    let (input, output): (&[&str], &[&str]) = match opcode {
        0x00 | 0x5b | 0xfe => (&[], &[]),
        0x01 => (&["a", "b"], &["a + b"]),
        0x02 => (&["a", "b"], &["a * b"]),
        0x03 => (&["a", "b"], &["a - b"]),
        0x04 | 0x05 => (&["a", "b"], &["a / b"]),
        0x06 | 0x07 => (&["a", "b"], &["a % b"]),
        0x08 => (&["a", "b", "m"], &["(a + b) % m"]),
        0x09 => (&["a", "b", "m"], &["(a * b) % m"]),
        0x0a => (&["a", "exponent"], &["a ** exponent"]),
        0x0b => (&["b", "value"], &["sign_extend(b, value)"]),
        0x10 | 0x12 => (&["a", "b"], &["a < b"]),
        0x11 | 0x13 => (&["a", "b"], &["a > b"]),
        0x14 => (&["a", "b"], &["a == b"]),
        0x15 => (&["a"], &["a == 0"]),
        0x16 => (&["a", "b"], &["a & b"]),
        0x17 => (&["a", "b"], &["a | b"]),
        0x18 => (&["a", "b"], &["a ^ b"]),
        0x19 => (&["a"], &["~a"]),
        0x1a => (&["index", "value"], &["byte(index, value)"]),
        0x1b => (&["shift", "value"], &["value << shift"]),
        0x1c | 0x1d => (&["shift", "value"], &["value >> shift"]),
        0x1e => (&["value"], &["leading_zero_bits(value)"]),
        0x20 => (&["offset", "size"], &["keccak256(memory range)"]),
        0x30 => (&[], &["address"]),
        0x31 => (&["address"], &["balance(address)"]),
        0x32 => (&[], &["origin"]),
        0x33 => (&[], &["caller"]),
        0x34 => (&[], &["call_value"]),
        0x35 => (&["offset"], &["calldata word at offset"]),
        0x36 => (&[], &["calldata_size"]),
        0x37 | 0x39 | 0x3e | 0x5e => (&["dest_offset", "src_offset", "size"], &[]),
        0x38 => (&[], &["code_size"]),
        0x3a => (&[], &["gas_price"]),
        0x3b => (&["address"], &["code_size(address)"]),
        0x3c => (&["address", "dest_offset", "src_offset", "size"], &[]),
        0x3d => (&[], &["returndata_size"]),
        0x3f => (&["address"], &["code_hash(address)"]),
        0x40 => (&["block_number"], &["block_hash"]),
        0x41 => (&[], &["fee_recipient"]),
        0x42 => (&[], &["timestamp"]),
        0x43 => (&[], &["block_number"]),
        0x44 => (&[], &["prev_randao"]),
        0x45 => (&[], &["gas_limit"]),
        0x46 => (&[], &["chain_id"]),
        0x47 => (&[], &["self_balance"]),
        0x48 => (&[], &["base_fee"]),
        0x49 => (&["index"], &["blob_versioned_hash"]),
        0x4a => (&[], &["blob_base_fee"]),
        0x4b => (&[], &["slot_number"]),
        0x50 => (&["value"], &[]),
        0x51 => (&["offset"], &["memory word at offset"]),
        0x52 | 0x53 => (&["offset", "value"], &[]),
        0x54 => (&["key"], &["storage[key]"]),
        0x55 | 0x5d => (&["key", "value"], &[]),
        0x56 => (&["destination"], &[]),
        0x57 => (&["destination", "condition"], &[]),
        0x58 => (&[], &["pc"]),
        0x59 => (&[], &["memory_size"]),
        0x5a => (&[], &["gas_remaining"]),
        0x5c => (&["key"], &["transient_storage[key]"]),
        0x5f => (&[], &["0"]),
        0x60..=0x7f => (&[], &["immediate"]),
        0xe6 => (&["s1", "...", "s[n]"], &["s[n]", "s1", "...", "s[n]"]),
        0xe7 => (&["s1", "...", "s[n+1]"], &["s[n+1]", "...", "s1"]),
        0xe8 => (
            &["s1", "...", "s[n+1]", "...", "s[m+1]"],
            &["s1", "...", "s[m+1]", "...", "s[n+1]"],
        ),
        0xf0 => (&["value", "offset", "size"], &["address or 0"]),
        0xf1 | 0xf2 => (
            &[
                "gas",
                "address",
                "value",
                "in_offset",
                "in_size",
                "out_offset",
                "out_size",
            ],
            &["success (0 or 1)"],
        ),
        0xf3 | 0xfd => (&["offset", "size"], &[]),
        0xf4 | 0xfa => (
            &[
                "gas",
                "address",
                "in_offset",
                "in_size",
                "out_offset",
                "out_size",
            ],
            &["success (0 or 1)"],
        ),
        0xf5 => (&["value", "offset", "size", "salt"], &["address or 0"]),
        0xff => (&["beneficiary"], &[]),
        _ => unreachable!("unsupported opcode"),
    };
    (
        input.iter().map(|value| (*value).to_string()).collect(),
        output.iter().map(|value| (*value).to_string()).collect(),
    )
}

fn stack_diagram(opcode: u8) -> String {
    let (input, output) = stack_io(opcode);
    let width = input.iter().map(String::len).max().unwrap_or(0).max(3) + 4;
    let mut diagram = format!("```text\n{:<width$}out:\n", "in:");
    for row in 0..input.len().max(output.len()) {
        let left = input.get(row).map(String::as_str).unwrap_or("");
        if let Some(right) = output.get(row) {
            writeln!(diagram, "{left:<width$}{right}").unwrap();
        } else {
            writeln!(diagram, "{left}").unwrap();
        }
    }
    write!(diagram, "{:<width$}...\n```", "...").unwrap();
    diagram
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
                assert!(entry.docs.contains("**Since:**"));
                assert!(entry.docs.contains("**Minimum gas:**"));
                assert!(entry.docs.contains("```text\nin:"));
                assert!(entry.docs.contains("out:\n"));
                let (input, output) = super::stack_io(opcode);
                if !matches!(opcode, 0x80..=0x9f | 0xe6..=0xe8) {
                    assert!(
                        entry.docs.contains(&format!(
                            "Pops {}, pushes {}.",
                            input.len(),
                            output.len()
                        )),
                        "stack counts for 0x{opcode:02x}"
                    );
                }
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
    fn diagrams_preserve_operand_order_and_stack_positions() {
        assert_eq!(
            super::stack_diagram(0x03),
            "```text\nin:    out:\na      a - b\nb\n...    ...\n```"
        );
        for (opcode, input, output) in [
            (0x04, vec!["a", "b"], vec!["a / b"]),
            (0x0a, vec!["a", "exponent"], vec!["a ** exponent"]),
            (0x1b, vec!["shift", "value"], vec!["value << shift"]),
            (0x57, vec!["destination", "condition"], vec![]),
            (
                0xf1,
                vec![
                    "gas",
                    "address",
                    "value",
                    "in_offset",
                    "in_size",
                    "out_offset",
                    "out_size",
                ],
                vec!["success (0 or 1)"],
            ),
            (0x80, vec!["s1"], vec!["s1", "s1"]),
            (0x81, vec!["s1", "s2"], vec!["s2", "s1", "s2"]),
            (0x90, vec!["s1", "s2"], vec!["s2", "s1"]),
            (0x91, vec!["s1", "s2", "s3"], vec!["s3", "s2", "s1"]),
            (
                0xa4,
                vec!["offset", "size", "topic1", "topic2", "topic3", "topic4"],
                vec![],
            ),
            (
                0xe6,
                vec!["s1", "...", "s[n]"],
                vec!["s[n]", "s1", "...", "s[n]"],
            ),
            (
                0xe7,
                vec!["s1", "...", "s[n+1]"],
                vec!["s[n+1]", "...", "s1"],
            ),
            (
                0xe8,
                vec!["s1", "...", "s[n+1]", "...", "s[m+1]"],
                vec!["s1", "...", "s[m+1]", "...", "s[n+1]"],
            ),
        ] {
            let (actual_input, actual_output) = super::stack_io(opcode);
            assert_eq!(actual_input, input, "input for 0x{opcode:02x}");
            assert_eq!(actual_output, output, "output for 0x{opcode:02x}");
        }
        for opcode in 0x80..=0x9f {
            let (input, output) = super::stack_io(opcode);
            if opcode < 0x90 {
                assert_eq!(output.len(), input.len() + 1);
                assert_eq!(output.first(), input.last());
                assert_eq!(&output[1..], input);
            } else {
                assert_eq!(output.len(), input.len());
                assert_eq!(output.first(), input.last());
                assert_eq!(output.last(), input.first());
                assert_eq!(output[1..output.len() - 1], input[1..input.len() - 1]);
            }
        }
    }

    #[test]
    fn fork_history_is_distinct_from_the_gas_schedule() {
        for (opcode, fork, gas) in [
            (0x03, "Frontier", 3),
            (0xf4, "Homestead", 100),
            (0x3d, "Byzantium", 2),
            (0xfd, "Byzantium", 0),
            (0x1b, "Constantinople", 3),
            (0x3f, "Constantinople", 100),
            (0xf5, "Constantinople", 32_000),
            (0x46, "Istanbul", 2),
            (0x47, "Istanbul", 5),
            (0x48, "London", 2),
            (0x44, "Paris (0x44 was DIFFICULTY since Frontier)", 2),
            (0x5f, "Shanghai", 2),
            (0x49, "Cancun", 3),
            (0x5c, "Cancun", 100),
            (0x5e, "Cancun", 3),
            (0x1e, "Osaka", 5),
            (0x4b, "Amsterdam (draft)", 2),
            (0xe6, "Amsterdam (draft)", 3),
            (0xe7, "Amsterdam (draft)", 3),
            (0xe8, "Amsterdam (draft)", 3),
        ] {
            let entry = info(opcode).unwrap();
            assert!(entry.docs.contains(&format!("**Since:** {fork}\n")));
            assert!(entry.docs.contains(&format!("**Minimum gas:** {gas} (")));
            let schedule = if matches!(opcode, 0x4b | 0xe6..=0xe8) {
                "Amsterdam draft"
            } else {
                "Osaka"
            };
            assert!(entry.docs.contains(&format!("({schedule}).")));
        }
        for (opcode, minimum, qualification) in [
            (0x0a, 10, "significant exponent byte"),
            (0x20, 30, "word hashed"),
            (0x31, 100, "cold account costs 2,600"),
            (0x3c, 100, "word copied"),
            (0x54, 100, "cold slot costs 2,100"),
            (0x55, 100, "more than 2,300 gas remaining"),
            (0xa0, 375, "topic charge"),
            (0xa4, 1_875, "topic charge"),
            (0xf0, 32_000, "initcode"),
            (0xf1, 100, "child execution"),
            (0xff, 5_000, "Cold beneficiary"),
        ] {
            let (gas, note) = super::minimum_gas(opcode);
            assert_eq!(gas, Some(minimum));
            assert!(note.contains(qualification), "0x{opcode:02x}: {note}");
        }
        assert_eq!(super::minimum_gas(0xfe).0, None);
        assert!(
            info(0xfe)
                .unwrap()
                .docs
                .contains("All remaining gas in this call")
        );
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
