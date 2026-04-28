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

## Controls

- `Tab` / `Shift+Tab`: Switch between bit manipulation, expression input, and history selection
- `m` / `M`: Cycle display mode forward or backward
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

Calculations are done on a 64-bit unsigned value. Addition, subtraction, and
multiplication wrap like normal low-level integer operations. Division by zero
and shifts of 64 bits or more are shown as errors. Bit range mask end indexes
must be no greater than 64.
