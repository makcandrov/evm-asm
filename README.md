# evm-asm

Compile-time EVM bytecode assembly macro returning `&'static [u8; N]`.

```rust
use evm_asm::bytecode;

const SIZE: [u8; 1] = [0x08];
const CODE: &[u8] = bytecode! {
    push1 #SIZE
    calldatasize
    eq
};

assert_eq!(CODE, &[0x60, 0x08, 0x36, 0x14]);
```

Enable optional opcode hover documentation in rust-analyzer with:

```toml
evm-asm = { version = "0.1", features = ["opcode-docs"] }
```

Disabled by default to avoid the extra expansion work. Only used opcodes are
documented; bytecode and runtime behavior are unchanged.

Implements legacy EVM opcode encodings through the
[Amsterdam execution-specs branch](https://github.com/ethereum/execution-specs/blob/forks/amsterdam/src/ethereum/forks/amsterdam/vm/instructions/__init__.py). Stack usage and jump
destinations are not validated.

Accepted grammar:

- Case-insensitive opcode names, separated by whitespace, commas, or semicolons.
  Rust `//` and `/* */` comments are accepted.
- `push1 value` through `push32 value`: one literal fitting the selected byte
  width, encoded big-endian and padded on the left with zeros.
- Exact PUSH interpolation: `push20 #ADDRESS`, `push20 #module::ADDRESS`, or
  `push20 #(ADDRESS.as_bytes())`. The constant or const expression must yield a
  byte array, slice, or reference with exactly the selected width, copied verbatim.
  Wrapper types need a const accessor; ordinary `AsRef`/`Deref` calls are unsupported.
- Left-padded PUSH interpolation: `push1 #~(SIZE.to_be_bytes())`,
  `push32 #~BYTES`, or `push32 #~left(BYTES)`. Adds leading zeros to shorter
  inputs and discards only zero leading bytes from longer inputs.
- Right-padded PUSH interpolation: `push32 #~right(BYTES)`. Adds trailing zeros
  to shorter inputs and discards only zero trailing bytes from longer inputs.
  Both modes preserve byte order and encode empty inputs as zero.
- `dupn n` and `swapn n`: `17 <= n <= 235`.
- `exchange n m`: `1 <= n < m` and `n + m <= 30`; separate operands with whitespace.
  Stack indices are encoded automatically.
- All other opcodes, including `push0`, `dup1`–`dup16`, `swap1`–`swap16`, and
  `log0`–`log4`, take no operands.
- Numeric operands are unsigned, unsuffixed integer literals: decimal, hexadecimal
  (`0x`), binary (`0b`), or octal (`0o`), with optional underscores.
  Runtime variables and labels are unsupported.
- Aliases: `sha3`/`keccak` for `keccak256`, `difficulty` for `prevrandao`, and
  `suicide` for `selfdestruct`.

Unknown opcodes, missing operands, overflows, and exact interpolation length
mismatches are compile errors.

Licensed under MIT or Apache-2.0, at your option.
