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

assert_eq!(CODE, b"\x60\x08\x36\x14");
```

Implements legacy EVM opcode encodings through the
[Amsterdam execution-specs branch](https://github.com/ethereum/execution-specs/blob/forks/amsterdam/src/ethereum/forks/amsterdam/vm/instructions/__init__.py). Stack usage and jump
destinations are not validated.

Accepted grammar:

- Case-insensitive opcode names, separated by whitespace, commas, or semicolons.
  Rust `//` and `/* */` comments are accepted.
- `push1 value` through `push32 value`: one literal fitting the selected byte
  width, encoded big-endian and padded on the left with zeros.
- PUSH interpolation: `push20 #ADDRESS`, `push20 #module::ADDRESS`, or
  `push20 #(ADDRESS.as_bytes())`. The constant or const expression must yield a
  byte array, slice, or reference with exactly the selected width, copied verbatim.
  Wrapper types need a const accessor; ordinary `AsRef`/`Deref` calls are unsupported.
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

Unknown opcodes, missing operands, out-of-range values, and interpolation length
mismatches are compile errors.
