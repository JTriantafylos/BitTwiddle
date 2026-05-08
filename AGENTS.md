# AGENTS.md

Guidance for coding agents working on BitTwiddle.

## Project Overview

BitTwiddle is a small Rust terminal UI for inspecting and manipulating unsigned
integer values in 8-bit, 16-bit, 32-bit, and 64-bit modes.
It is implemented as a Ratatui app with the terminal event loop in
`src/main.rs`, shared message/update dispatch in `src/lib.rs`, rendering and
layout helpers in `src/view.rs`, model state and state transitions in
`src/model.rs`, and expression parsing/computation in `src/expression.rs`.

Core behavior:

- Expression input accepts unsigned integer literals in decimal, hex (`0x`), octal
  (`0o`), and binary (`0b`) forms.
- A single unsigned integer literal sets the current model value directly.
- Supported operators are `+`, `-`, `*`, `/`, `%`, `<<`, `>>`, `|`, `&`, and `^`.
- Unary bitwise NOT is supported as `~literal`; bare `~` applies to the current
  model value.
- Bit range masks are supported as `literal[start:end]` or `literal[index]`;
  omitted literals use the current model value. Ranges are half-open and
  zero-based, so `[0:2]` keeps bits 0 and 1, while `[4]` keeps only bit 4.
- Arithmetic wraps within the active integer width for add/subtract/multiply.
- Division by zero and shifts greater than or equal to the active integer width
  are rejected with errors.
- If the left operand is omitted in an expression, the current model value is
  used as operand 1.
- The UI has expression input, bit manipulation, display mode, and calculation
  history state.

## Repository Layout

- `Cargo.toml`: package metadata and dependencies.
- `src/lib.rs`: library crate root, shared `Message` type, terminal key/mouse
  mapping, and update dispatch.
- `src/main.rs`: binary entry point, terminal lifecycle, event reading, and the
  draw/update loop.
- `src/view.rs`: rendering, layout, scrolling, and hit-testing helpers.
- `src/model.rs`: model state, state transitions, input/history behavior, and
  display/history formatting helpers.
- `src/expression.rs`: expression parsing, operators, and computation logic.

The package has both a library crate and a binary target. Unit tests live
alongside the library modules; `src/main.rs` should stay thin.

## Commands

Use these from the repository root:

- `cargo fmt`: format Rust code.
- `cargo check`: fast compile/type check.
- `cargo test`: run the unit test suite.
- `cargo run`: launch the TUI locally.

Run `cargo fmt` and `cargo check` after code changes. Run `cargo test` when
adding tests or changing parsing, computation, cursor movement, history, or bit
selection behavior.

## Architecture Notes

The current code follows a simple Elm-style loop:

- `BitTwiddleModel` holds all app state.
- `Message` represents events and internal state transitions.
- `handle_event` in `src/main.rs` reads terminal events into raw messages.
- `update` in `src/lib.rs` mutates the model and can return follow-up messages.
- `view` in `src/view.rs` renders the current model with Ratatui widgets.
- `main` initializes Ratatui, runs the draw/event/update loop, then restores the
  terminal.

Prefer preserving this message/update structure while refactoring. Extract
focused helpers or small methods before adding larger abstractions.

## Editing Guidance

- Keep changes narrow. This project is intentionally compact, and broad rewrites
  make behavior harder to verify.
- Keep documentation current when changing behavior, commands, dependencies,
  architecture, or user-facing workflows. Update `README.md`, `AGENTS.md`, and
  any other relevant docs in the same change when they would otherwise become
  stale.
- Prefer named structs or helper functions when they remove tuple ambiguity or
  duplicated formatting/rendering logic.
- Keep parser and computation behavior explicit. Do not silently change numeric
  bases, omitted-left-operand semantics, wrapping arithmetic, or invalid-shift
  handling.
- Use `OnceLock` or another one-time initialization strategy for compiled
  regexes; do not compile regexes on every expression submission.
- `expression_input_cursor_index` is currently a byte index into `String`.
  Before accepting arbitrary character input, make cursor movement and editing
  Unicode-safe. The lower-risk alternative is to restrict accepted input to
  ASCII digits, operators, prefixes, and whitespace.
- `view` should be read-only over time. If changing rendering, prefer helpers
  that take immutable model data.
- Terminal restoration matters. If changing `main`, drawing, or event handling,
  ensure `ratatui::restore()` still runs on early errors.

## UI Behavior To Preserve

- `Tab` and `BackTab` cycle input modes.
- `m` and `M` cycle display modes forward and backward.
- `w` and `W` cycle integer width modes forward and backward.
- `Ctrl+C` clears the current value, error, and expression input.
- `Esc` dismisses errors.
- `q` opens quit confirmation.
- In quit confirmation:
  - `y` and `Y` quit.
  - Any other key cancels quit confirmation.
- In expression input mode:
  - `Enter` submits the expression.
  - Left/right move the input cursor.
  - Home/end jump to the start/end.
  - Up/down navigate expression input history.
  - Backspace/delete edit the current input.
- In bit manipulation mode:
  - Left/right move selected bit by one bit.
  - Up/down move selected bit by 32 bits.
  - Space toggles the selected bit.
- Left-clicking a bit in the bit manipulation area toggles that bit and enters
  bit manipulation mode.
- In history selection mode:
  - Up/down move the selected calculation history entry.
  - Enter copies the selected calculation history entry into expression input.

## Testing Priorities

When adding tests, start with pure behavior that can be tested without a
terminal:

- `parse_unsigned` for decimal, hex, octal, and binary literals.
- `parse_expr` for explicit operands and omitted left operands.
- `parse_expr` and expression computation for unary NOT, half-open bit range
  masks, and single-bit range mask shorthand.
- `compute` for wrapping arithmetic, bitwise operators, division by zero, and
  invalid shifts.
- History navigation in `update`.
- Bit selection clamping and toggling.
- Display-mode formatting once extracted into a helper.
- Bit visualization ordering once extracted into a helper.

For TUI changes, keep rendering helpers deterministic where possible so they can
be unit-tested without driving an interactive terminal.

## Dependencies

Current runtime dependencies:

- `ratatui`: terminal UI, including Crossterm event integration.
- `regex`: expression parsing.

Avoid adding dependencies for small helper logic unless they clearly reduce
complexity or match an existing project direction.
