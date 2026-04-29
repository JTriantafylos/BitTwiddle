# BitTwiddle

BitTwiddle is a small terminal programmer calculator.

It is built for quick base conversions, bitwise math, shifts, masks, and flag
toggling without leaving the terminal. Type an expression, inspect the bits,
switch number bases, or move around the bit grid and flip individual bits by
hand.

## Building & Running

```sh
cargo build
cargo run
```

## Supported Input

Expression input accepts ASCII expression syntax only: digits, hex letters,
base prefix letters, operators, brackets, colons, and ASCII whitespace. Other
typed characters are ignored.

Numbers can be written in the usual programmer-friendly formats:

```text
42
0x2A
0o755
0b101010
```

A single number sets the current value directly:

```text
32
```

Supported operators:

- `+`: Addition
- `-`: Subtraction
- `*`: Multiplication
- `/`: Division
- `<<`: Binary Left-shift
- `>>`: Binary Right-shift
- `|`: Binary OR
- `&`: Binary AND
- `^`: Binary XOR
- `~`: Bitwise NOT

Expressions currently use one operation at a time: one binary operation, one
unary NOT operation, or one bit range mask. Unary NOT and bit range masks are
standalone expression forms, not higher-precedence operators inside larger
expressions. Inputs such as `1 + 2 * 3`, `~1 + 2`, `33[0:2] | 4`, and
parenthesized expressions are rejected. Submit each step separately, using the
current value as the implicit operand when that is useful.

Examples:

```text
0xFF & 0b10101010
1 << 12
0xF0 | 0x0F
~5
```

You can also skip the left side of an expression. In that case, BitTwiddle uses
the current calculator value as the first operand:

```text
+ 1
<< 4
& 0xFF
```

Unary NOT can omit its operand to use the current calculator value:

```text
~
```

Bit range masks use half-open, zero-based indexes. For example, `[0:2]` keeps
bits 0 and 1 and clears every other bit. A single index keeps only that bit:

```text
33[0:2]
33[4]
0b1011[1:3]
[0:8]
[4]
```

BitTwiddle supports 8-bit, 16-bit, 32-bit, and 64-bit unsigned modes. The active
width controls arithmetic wrapping, bitwise NOT, shift limits, bit range bounds,
the visible bit grid, and the stored value. Switching to a narrower width masks
away higher bits immediately. The signed display mode shows values using the
active width's two's-complement interpretation.

## Controls

- `Tab` / `Shift+Tab`: Switch between bit manipulation, expression input, and history selection
- `m` / `M`: Cycle display mode forward or backward
- `w` / `W`: Cycle integer width forward or backward
- `Ctrl+C`: Clear the current value
- `Esc`: Dismiss an error
- `q`: Quit
- Left-click the input box: Switch to expression input mode and move the cursor

When editing bits:

- `Up` / `Down` / `Left` / `Right`: Select bit
- `Space`: Toggle selected bit
- Left-click a bit: Switch to bit manipulation mode and toggle that bit

When selecting history:

- `Up` / `Down`: Select a history item
- `Enter`: Copy the selected expression into the input box
- Left-click a history item: Switch to history selection mode and select that item

## Usage Notes

Calculations are done on the active unsigned integer width. Addition,
subtraction, and multiplication wrap like normal low-level integer operations.
Division by zero and shifts greater than or equal to the active width are shown
as errors. Bit range mask end indexes must be no greater than the active width.
