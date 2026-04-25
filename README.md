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

Examples:

```text
0xFF & 0b10101010
1 << 12
0xF0 | 0x0F
```

You can also skip the left side of an expression. In that case, BitTwiddle uses
the current calculator value as the first operand:

```text
+ 1
<< 4
& 0xFF
```

## Controls

```text
Tab / Shift+Tab - Switch between bit manipulation, expression input, and input history
m / M           - Cycle display mode forward or backward
c               - Clear the current value
Esc             - Dismiss an error
q               - Quit
```

When editing bits:

```text
Up / Down / Left / Right - Select bit
Space                    - Toggle selected bit
```

## Usage Notes

Calculations are done on a 64-bit unsigned value. Addition, subtraction, and
multiplication wrap like normal low-level integer operations. Division by zero
and shifts of 64 bits or more are shown as errors.
