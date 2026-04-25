# AGENTS.md

Guidance for coding agents working on BitTwiddle.

## Project Overview

BitTwiddle is a small Rust terminal UI for inspecting and manipulating a `u64`.
It is implemented as a Ratatui app in `src/main.rs`.

Core behavior:

- Expression input accepts unsigned integer literals in decimal, hex (`0x`), octal
  (`0o`), and binary (`0b`) forms.
- Supported operators are `+`, `-`, `*`, `/`, `<<`, `>>`, `|`, `&`, and `^`.
- Arithmetic uses `u64` wrapping behavior for add/subtract/multiply.
- Division by zero and shifts of 64 bits or more are rejected with errors.
- If the left operand is omitted in an expression, the current model value is
  used as operand 1.
- The UI has expression input, bit manipulation, display mode, and calculation
  history state.

## Repository Layout

- `Cargo.toml`: package metadata and dependencies.
- `src/main.rs`: all current application logic, rendering, event handling, and
  model state.

There is no library crate yet and no current test suite.

## Commands

Use these from the repository root:

- `cargo fmt`: format Rust code.
- `cargo check`: fast compile/type check.
- `cargo test`: run tests. This currently reports 0 tests, but it is still the
  right command once tests are added.
- `cargo run`: launch the TUI locally.

Run `cargo fmt` and `cargo check` after code changes. Run `cargo test` when
adding tests or changing parsing, computation, cursor movement, history, or bit
selection behavior.

## Architecture Notes

The current code follows a simple Elm-style loop:

- `BitTwiddleModel` holds all app state.
- `Message` represents events and internal state transitions.
- `handle_event` maps terminal events into messages.
- `update` mutates the model and can return follow-up messages.
- `view` renders the current model with Ratatui widgets.
- `main` initializes Ratatui, runs the draw/event/update loop, then restores the
  terminal.

Prefer preserving this message/update structure while refactoring. Extract
focused helpers or small methods before adding larger abstractions.

## Editing Guidance

- Keep changes narrow. This project is intentionally compact, and broad rewrites
  make behavior harder to verify.
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
- `c` clears the current value and error.
- `Esc` dismisses errors.
- `q` quits.
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

## Testing Priorities

When adding tests, start with pure behavior that can be tested without a
terminal:

- `parse_unsigned` for decimal, hex, octal, and binary literals.
- `parse_expr` for explicit operands and omitted left operands.
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
