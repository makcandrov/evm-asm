# evm-asm

Compile-time EVM bytecode assembly macro.

```rust
use evm_asm::bytecode;

const CODE: &[u8] = bytecode! {
    push1 0x08
    calldatasize
    eq
};

assert_eq!(CODE, b"\x60\x08\x36\x14");
```

Enable `alloy` to use `alloy_bytecode!` with the same syntax and return
`alloy_primitives::Bytes`.

```toml
[dependencies]
evm-asm = { version = "0.1", features = ["alloy"] }
```

Implements legacy EVM opcode encodings through the
[Amsterdam execution-specs branch](https://github.com/ethereum/execution-specs/blob/forks/amsterdam/src/ethereum/forks/amsterdam/vm/instructions/__init__.py). Stack usage and jump
destinations are not validated.

Accepted grammar:

- Case-insensitive opcode names, separated by whitespace, commas, or semicolons.
  Rust `//` and `/* */` comments are accepted.
- `push1 value` through `push32 value`: one literal fitting the selected byte
  width, encoded big-endian and padded on the left with zeros.
- `dupn n` and `swapn n`: `17 <= n <= 235`.
- `exchange n m`: `1 <= n < m` and `n + m <= 30`; separate operands with whitespace.
  Stack indices are encoded automatically.
- All other opcodes, including `push0`, `dup1`–`dup16`, `swap1`–`swap16`, and
  `log0`–`log4`, take no literal operands.
- Operands are unsigned, unsuffixed integer literals: decimal, hexadecimal
  (`0x`), binary (`0b`), or octal (`0o`), with optional underscores. No variables,
  expressions, or labels.
- Aliases: `sha3`/`keccak` for `keccak256`, `difficulty` for `prevrandao`, and
  `suicide` for `selfdestruct`.

Unknown opcodes, missing operands, and out-of-range values are compile errors.
