use std::{
    ops::{BitAnd, BitOr, BitXor},
    sync::OnceLock,
};

use regex::Regex;

use ratatui::{
    Frame,
    crossterm::event::{self, Event, KeyCode, KeyEvent},
    layout::{Constraint, Layout, Position},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, List, ListItem, Paragraph},
};

/// Parses an unsigned integer literal into a `u64`.
///
/// Decimal literals are accepted by default. Prefixes select other bases:
/// `0x`/`0X` for hexadecimal, `0o`/`0O` for octal, and `0b`/`0B` for binary.
/// Returns the underlying integer parse error for empty, invalid, or overflowing
/// literals.
fn parse_unsigned(s: &str) -> Result<u64, std::num::ParseIntError> {
    if let Some(rest) = s.to_lowercase().strip_prefix("0x") {
        u64::from_str_radix(rest, 16)
    } else if let Some(rest) = s.to_lowercase().strip_prefix("0o") {
        u64::from_str_radix(rest, 8)
    } else if let Some(rest) = s.to_lowercase().strip_prefix("0b") {
        u64::from_str_radix(rest, 2)
    } else {
        u64::from_str_radix(s, 10)
    }
}

/// Parses an arithmetic or bitwise expression into `(operand1, operand2, operator)`.
///
/// Operands are unsigned integer literals accepted by [`parse_unsigned`]. When the
/// left operand is omitted, the current model value is used as `operand1`.
/// Returns an error for malformed expressions or operands that cannot fit in a
/// `u64`.
fn parse_expr(model: &BitTwiddleModel, input: &str) -> Result<(u64, u64, String), String> {
    static EXPR_RE: OnceLock<Regex> = OnceLock::new();
    let re = EXPR_RE.get_or_init(|| {
        Regex::new(concat!(
            r"^",                                                           // Beginning marker
            r"\s*", // Match any preceding whitespace
            r"(?<operand1>0[xX][0-9a-fA-F]+|0[oO][0-7]+|0[bB][0-1]+|\d+)?", // Capture the first operand
            r"\s*",                                                         // Match any whitespace
            r"(?<operator>\+|-|\*|\/|<<|>>|\||\&|\^+)",                     // Capture the operator
            r"\s*",                                                         // Match any whitespace
            r"(?<operand2>0[xX][0-9a-fA-F]+|0[oO][0-7]+|0[bB][0-1]+|\d+)", // Capture the second operand
            r"\s*", // Match any trailing whitespace
            r"$"    // End marker
        ))
        .unwrap()
    });
    let captures = re.captures(input);
    match captures {
        Some(caps) => {
            let operand1_match = caps.name("operand1");
            let operand1: u64 = match operand1_match {
                Some(s) => parse_unsigned(s.as_str())
                    .map_err(|e| format!("Error parsing operand 1 - {}", e))?,
                None => model.value,
            };
            let operand2: u64 = parse_unsigned(&caps["operand2"])
                .map_err(|e| format!("Error parsing operand 2 - {}", e))?;
            let operator = String::from(&caps["operator"]);
            Ok((operand1, operand2, operator))
        }
        None => Err(format!("Malformed expression '{}'", input)),
    }
}

fn compute(operand1: u64, operand2: u64, operator: &str) -> Result<u64, String> {
    match operator {
        "+" => Ok(operand1.wrapping_add(operand2)),
        "-" => Ok(operand1.wrapping_sub(operand2)),
        "*" => Ok(operand1.wrapping_mul(operand2)),
        "/" => operand1
            .checked_div(operand2)
            .ok_or("Division by zero".into()),
        "<<" => {
            if operand2 >= u64::BITS.into() {
                Err(format!("Invalid left-shift: {}", operand2))
            } else {
                Ok(operand1 << operand2 as u32)
            }
        }
        ">>" => {
            if operand2 >= u64::BITS.into() {
                Err(format!("Invalid right-shift: {}", operand2))
            } else {
                Ok(operand1 >> operand2 as u32)
            }
        }
        "|" => Ok(operand1.bitor(operand2)),
        "&" => Ok(operand1.bitand(operand2)),
        "^" => Ok(operand1.bitxor(operand2)),
        _ => Err(format!("Unsupported operator: {}", operator)),
    }
}

#[derive(Default, PartialEq, Eq)]
enum RunningState {
    #[default]
    Running,
    Done,
}

#[derive(Default, PartialEq, Eq)]
enum DisplayMode {
    #[default]
    Decimal,
    Hex,
    Octal,
}

impl DisplayMode {
    fn prev(&self) -> Self {
        match *self {
            DisplayMode::Decimal => DisplayMode::Octal,
            DisplayMode::Hex => DisplayMode::Decimal,
            DisplayMode::Octal => DisplayMode::Hex,
        }
    }
    fn next(&self) -> Self {
        match *self {
            DisplayMode::Decimal => DisplayMode::Hex,
            DisplayMode::Hex => DisplayMode::Octal,
            DisplayMode::Octal => DisplayMode::Decimal,
        }
    }
}

#[derive(Default, PartialEq, Eq)]
enum InputMode {
    BitManipulation,
    #[default]
    ExpressionInput,
    HistorySelect,
}

impl InputMode {
    fn prev(&self) -> Self {
        match *self {
            InputMode::BitManipulation => InputMode::HistorySelect,
            InputMode::ExpressionInput => InputMode::BitManipulation,
            InputMode::HistorySelect => InputMode::ExpressionInput,
        }
    }
    fn next(&self) -> Self {
        match *self {
            InputMode::BitManipulation => InputMode::ExpressionInput,
            InputMode::ExpressionInput => InputMode::HistorySelect,
            InputMode::HistorySelect => InputMode::BitManipulation,
        }
    }
}

struct BitTwiddleModel {
    running_state: RunningState,
    value: u64,
    error: Option<String>,
    display_mode: DisplayMode,
    input_mode: InputMode,
    expression_input_text: String,
    expression_input_cursor_index: usize,
    expression_input_history: Vec<String>,
    expression_input_history_index: Option<usize>,
    calculation_history: Vec<(u64, String, u64, u64)>,
    selected_bit_index: usize,
}

impl Default for BitTwiddleModel {
    fn default() -> Self {
        Self {
            running_state: Default::default(),
            value: 0,
            error: None,
            display_mode: Default::default(),
            input_mode: Default::default(),
            expression_input_text: Default::default(),
            expression_input_cursor_index: Default::default(),
            expression_input_history: Vec::default(),
            expression_input_history_index: None,
            calculation_history: Vec::default(),
            selected_bit_index: Default::default(),
        }
    }
}

enum Message {
    KeyPressed(KeyEvent),
    ClearInput,
    SubmitExpression,
    ShiftInputCursor(isize),
    ShiftSelectedBit(isize),
    ToggleSelectedBit,
    PrevInputMode,
    NextInputMode,
    PrevInputHistory,
    NextInputHistory,
    PrevDisplayMode,
    NextDisplayMode,
    DismissError,
    Quit,
}

fn update(model: &mut BitTwiddleModel, msg: Message) -> Option<Message> {
    match msg {
        Message::KeyPressed(key) => match key.code {
            KeyCode::BackTab => Some(Message::PrevInputMode),
            KeyCode::Tab => Some(Message::NextInputMode),
            KeyCode::Char('M') => Some(Message::PrevDisplayMode),
            KeyCode::Char('m') => Some(Message::NextDisplayMode),
            KeyCode::Char('c') => Some(Message::ClearInput),
            KeyCode::Esc => Some(Message::DismissError),

            KeyCode::Left if model.input_mode == InputMode::BitManipulation => {
                Some(Message::ShiftSelectedBit(1))
            }
            KeyCode::Right if model.input_mode == InputMode::BitManipulation => {
                Some(Message::ShiftSelectedBit(-1))
            }
            KeyCode::Up if model.input_mode == InputMode::BitManipulation => {
                Some(Message::ShiftSelectedBit(32))
            }
            KeyCode::Down if model.input_mode == InputMode::BitManipulation => {
                Some(Message::ShiftSelectedBit(-32))
            }
            KeyCode::Char(' ') if model.input_mode == InputMode::BitManipulation => {
                Some(Message::ToggleSelectedBit)
            }

            KeyCode::Enter if model.input_mode == InputMode::ExpressionInput => {
                Some(Message::SubmitExpression)
            }
            KeyCode::Left if model.input_mode == InputMode::ExpressionInput => {
                Some(Message::ShiftInputCursor(-1))
            }
            KeyCode::Right if model.input_mode == InputMode::ExpressionInput => {
                Some(Message::ShiftInputCursor(1))
            }
            KeyCode::Home if model.input_mode == InputMode::ExpressionInput => {
                Some(Message::ShiftInputCursor(isize::MIN))
            }
            KeyCode::End if model.input_mode == InputMode::ExpressionInput => {
                Some(Message::ShiftInputCursor(isize::MAX))
            }
            KeyCode::Up if model.input_mode == InputMode::ExpressionInput => {
                Some(Message::NextInputHistory)
            }
            KeyCode::Down if model.input_mode == InputMode::ExpressionInput => {
                Some(Message::PrevInputHistory)
            }
            KeyCode::Backspace if model.input_mode == InputMode::ExpressionInput => {
                if !model.expression_input_text.is_empty()
                    && model.expression_input_cursor_index > 0
                {
                    model
                        .expression_input_text
                        .remove(model.expression_input_cursor_index - 1);
                    Some(Message::ShiftInputCursor(-1))
                } else {
                    None
                }
            }
            KeyCode::Delete if model.input_mode == InputMode::ExpressionInput => {
                if !model.expression_input_text.is_empty()
                    && model.expression_input_cursor_index < model.expression_input_text.len()
                {
                    model
                        .expression_input_text
                        .remove(model.expression_input_cursor_index);
                }
                None
            }
            KeyCode::Char(c) if model.input_mode == InputMode::ExpressionInput => {
                model
                    .expression_input_text
                    .insert(model.expression_input_cursor_index, c);
                Some(Message::ShiftInputCursor(1))
            }

            _ => None,
        },
        Message::ClearInput => {
            model.value = 0;
            model.error = None;
            None
        }
        Message::SubmitExpression => {
            match parse_expr(model, &model.expression_input_text) {
                Ok((operand1, operand2, operator)) => {
                    match compute(operand1, operand2, &operator) {
                        Ok(result) => {
                            model.value = result;
                            model.error = None;
                            model
                                .expression_input_history
                                .push(model.expression_input_text.clone());
                            model
                                .calculation_history
                                .push((operand1, operator, operand2, result));
                        }
                        Err(e) => {
                            model.error = Some(format!("Computation error: {}", e));
                        }
                    }
                }
                Err(e) => {
                    model.error = Some(format!("Parse error: {}", e));
                }
            };
            model.expression_input_cursor_index = 0;
            model.expression_input_text.clear();
            model.expression_input_history_index = None;
            None
        }
        Message::ShiftInputCursor(offset) => {
            model.expression_input_cursor_index = model
                .expression_input_cursor_index
                .saturating_add_signed(offset)
                .clamp(0, model.expression_input_text.len());
            None
        }
        Message::ShiftSelectedBit(offset) => {
            model.selected_bit_index = model
                .selected_bit_index
                .saturating_add_signed(offset)
                .clamp(0, 63);
            None
        }
        Message::ToggleSelectedBit => {
            model.value ^= 1_u64 << model.selected_bit_index;
            model.error = None;
            None
        }
        Message::PrevInputMode => {
            model.input_mode = model.input_mode.prev();
            None
        }
        Message::NextInputMode => {
            model.input_mode = model.input_mode.next();
            None
        }
        Message::PrevInputHistory => {
            match model.expression_input_history_index {
                Some(0) => {
                    model.expression_input_text = String::default();
                    model.expression_input_cursor_index = 0;
                    model.expression_input_history_index = None;
                }
                Some(n) if !model.expression_input_history.is_empty() => {
                    let index = n
                        .saturating_sub(1)
                        .clamp(0, model.expression_input_history.len() - 1);
                    model.expression_input_text = model
                        .expression_input_history
                        .iter()
                        .rev()
                        .nth(index)
                        .unwrap()
                        .clone();
                    model.expression_input_cursor_index = model.expression_input_text.len();
                    model.expression_input_history_index = Some(index);
                }
                _ => {}
            };
            None
        }
        Message::NextInputHistory => {
            if !model.expression_input_history.is_empty() {
                let index = match model.expression_input_history_index {
                    None => 0,
                    Some(n) => n
                        .saturating_add(1)
                        .clamp(0, model.expression_input_history.len() - 1),
                };
                model.expression_input_text = model
                    .expression_input_history
                    .iter()
                    .rev()
                    .nth(index)
                    .unwrap()
                    .clone();
                model.expression_input_cursor_index = model.expression_input_text.len();
                model.expression_input_history_index = Some(index);
            }
            None
        }
        Message::PrevDisplayMode => {
            model.display_mode = model.display_mode.prev();
            None
        }
        Message::NextDisplayMode => {
            model.display_mode = model.display_mode.next();
            None
        }
        Message::DismissError => {
            model.error = None;
            None
        }
        Message::Quit => {
            model.running_state = RunningState::Done;
            None
        }
    }
}

fn view(model: &mut BitTwiddleModel, frame: &mut Frame) {
    let vertical = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(4),
        Constraint::Length(3),
        Constraint::Min(1),
        Constraint::Length(1),
    ]);
    let [
        value_areas,
        bit_area,
        input_area,
        expression_history_area,
        help_area,
    ] = vertical.areas(frame.area());

    let [value_area, mode_area] =
        Layout::horizontal([Constraint::Min(1), Constraint::Length(5)]).areas(value_areas);

    let val_text = Text::from(match &model.error {
        Some(err) => err.to_string(),
        None => match model.display_mode {
            DisplayMode::Decimal => format!("{}", model.value),
            DisplayMode::Hex => format!("{:#X}", model.value),
            DisplayMode::Octal => format!("{:#o}", model.value),
        },
    });
    frame.render_widget(
        Paragraph::new(val_text).block(Block::bordered()).centered(),
        value_area,
    );

    let dec_indicator = Span::styled(
        "D",
        match model.display_mode {
            DisplayMode::Decimal => Style::default().underlined(),
            _ => Style::default().dim(),
        },
    );
    let hex_indicator = Span::styled(
        "H",
        match model.display_mode {
            DisplayMode::Hex => Style::default().underlined(),
            _ => Style::default().dim(),
        },
    );
    let oct_indicator = Span::styled(
        "O",
        match model.display_mode {
            DisplayMode::Octal => Style::default().underlined(),
            _ => Style::default().dim(),
        },
    );
    let mode_widget = Paragraph::new(Line::from(vec![
        dec_indicator,
        hex_indicator,
        oct_indicator,
    ]))
    .block(Block::bordered());
    frame.render_widget(mode_widget, mode_area);

    let input = Paragraph::new(model.expression_input_text.as_str()).block(
        Block::bordered()
            .border_style(match model.input_mode {
                InputMode::ExpressionInput => Style::default().fg(Color::Yellow),
                _ => Style::default(),
            })
            .title("Input"),
    );
    frame.render_widget(input, input_area);
    if model.input_mode == InputMode::ExpressionInput {
        frame.set_cursor_position(Position::new(
            // Draw the cursor at the current position in the input field.
            // This position is can be controlled via the left and right arrow key
            input_area.x + model.expression_input_cursor_index as u16 + 1,
            // Move one line down, from the border to the input line
            input_area.y + 1,
        ))
    }

    const BITS_PER_LINE: usize = 32;
    const BITS_PER_WORD: usize = 16;
    const BITS_PER_NIBBLE: usize = 4;
    let display_value = model.value;
    let bit_manipulation_mode = model.input_mode == InputMode::BitManipulation;
    let selected_bit_index = model.selected_bit_index;
    let selected_bit_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::REVERSED);
    let bit_visualization_text = (0..2)
        .rev()
        .map(|line_num| {
            Line::from(
                (0..2)
                    .rev()
                    .flat_map(|word_num| {
                        vec![
                            Span::styled(
                                (((line_num * BITS_PER_LINE) + (word_num * BITS_PER_WORD))
                                    + (BITS_PER_WORD - 1))
                                    .to_string(),
                                Style::default().bold(),
                            ),
                            Span::raw("  "),
                        ]
                        .into_iter()
                        .chain((0..4).rev().flat_map(move |nibble_num| {
                            (0..4)
                                .rev()
                                .flat_map(move |nibble_index| {
                                    let bit_index = (line_num * BITS_PER_LINE)
                                        + (word_num * BITS_PER_WORD)
                                        + (nibble_num * BITS_PER_NIBBLE)
                                        + nibble_index;
                                    let bit = if (display_value >> bit_index) & 1 == 1 {
                                        "1"
                                    } else {
                                        "0"
                                    };
                                    let bit_style = if bit_manipulation_mode
                                        && bit_index == selected_bit_index
                                    {
                                        selected_bit_style
                                    } else {
                                        Style::default()
                                    };
                                    let separator = if nibble_index == 0 {
                                        if nibble_num == 0 { "" } else { "  " }
                                    } else {
                                        " "
                                    };
                                    [Span::styled(bit, bit_style), Span::raw(separator)]
                                })
                                .collect::<Vec<Span>>()
                        }))
                        .chain([Span::raw("  ")])
                    })
                    .collect::<Vec<Span>>(),
            )
        })
        .collect::<Vec<Line>>();

    let bit_visualization = Paragraph::new(bit_visualization_text)
        .block(
            Block::bordered()
                .border_style(match model.input_mode {
                    InputMode::BitManipulation => Style::default().fg(Color::Yellow),
                    _ => Style::default(),
                })
                .title("Bits"),
        )
        .centered();
    frame.render_widget(bit_visualization, bit_area);

    let expression_history =
        List::new(
            model
                .calculation_history
                .iter()
                .rev()
                .map(|e| match model.display_mode {
                    DisplayMode::Decimal => {
                        ListItem::new(format!("{} {} {} = {}", e.0, e.1, e.2, e.3))
                    }
                    DisplayMode::Hex => {
                        ListItem::new(format!("{:#X} {} {:#X} = {:#X}", e.0, e.1, e.2, e.3))
                    }
                    DisplayMode::Octal => {
                        ListItem::new(format!("{:#o} {} {:#o} = {:#o}", e.0, e.1, e.2, e.3))
                    }
                }),
        )
        .block(
            Block::bordered()
                .border_style(match model.input_mode {
                    InputMode::HistorySelect => Style::default().fg(Color::Yellow),
                    _ => Style::default(),
                })
                .title("History"),
        );
    frame.render_widget(expression_history, expression_history_area);

    let help_text = Line::from(
        vec![
            "[c] Clear",
            "[m] Cycle display mode (forward)",
            "[M] Cycle display mode (backward)",
            "[arrows] Select bit",
            "[space] Toggle bit",
        ]
        .join(" "),
    );
    frame.render_widget(Paragraph::new(help_text).centered(), help_area);
}

fn handle_event() -> Result<Option<Message>, std::io::Error> {
    if let Event::Key(key) = event::read()? {
        match key.code {
            KeyCode::Char('q') => Ok(Some(Message::Quit)),
            _ => Ok(Some(Message::KeyPressed(key))),
        }
    } else {
        Ok(None)
    }
}

fn main() -> Result<(), std::io::Error> {
    let mut terminal = ratatui::init();
    let result = (|| {
        let mut model: BitTwiddleModel = BitTwiddleModel::default();

        while model.running_state != RunningState::Done {
            // Render the current view
            terminal.draw(|f| view(&mut model, f))?;

            // Handle events and map to a Message
            let mut current_msg = handle_event()?;

            // Process updates as long as they return a non-None message
            while current_msg.is_some() {
                current_msg = update(&mut model, current_msg.unwrap());
            }
        }

        Ok(())
    })();

    ratatui::restore();
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::num::IntErrorKind;

    fn model_with_value(value: u64) -> BitTwiddleModel {
        BitTwiddleModel {
            value,
            ..Default::default()
        }
    }

    #[test]
    fn parse_unsigned_accepts_decimal_literals() {
        assert_eq!(parse_unsigned("0"), Ok(0));
        assert_eq!(parse_unsigned("42"), Ok(42));
        assert_eq!(parse_unsigned("18446744073709551615"), Ok(u64::MAX));
    }

    #[test]
    fn parse_unsigned_accepts_hex_literals() {
        assert_eq!(parse_unsigned("0x0"), Ok(0));
        assert_eq!(parse_unsigned("0xff"), Ok(255));
        assert_eq!(parse_unsigned("0XFF"), Ok(255));
        assert_eq!(parse_unsigned("0xffffffffffffffff"), Ok(u64::MAX));
    }

    #[test]
    fn parse_unsigned_accepts_octal_literals() {
        assert_eq!(parse_unsigned("0o0"), Ok(0));
        assert_eq!(parse_unsigned("0o17"), Ok(15));
        assert_eq!(parse_unsigned("0O17"), Ok(15));
        assert_eq!(parse_unsigned("0o1777777777777777777777"), Ok(u64::MAX));
    }

    #[test]
    fn parse_unsigned_accepts_binary_literals() {
        assert_eq!(parse_unsigned("0b0"), Ok(0));
        assert_eq!(parse_unsigned("0b1010"), Ok(10));
        assert_eq!(parse_unsigned("0B1010"), Ok(10));
        assert_eq!(
            parse_unsigned("0b1111111111111111111111111111111111111111111111111111111111111111"),
            Ok(u64::MAX)
        );
    }

    #[test]
    fn parse_unsigned_rejects_invalid_digits_for_base() {
        assert_eq!(
            *parse_unsigned("0xg").unwrap_err().kind(),
            IntErrorKind::InvalidDigit
        );
        assert_eq!(
            *parse_unsigned("0o8").unwrap_err().kind(),
            IntErrorKind::InvalidDigit
        );
        assert_eq!(
            *parse_unsigned("0b2").unwrap_err().kind(),
            IntErrorKind::InvalidDigit
        );
        assert_eq!(
            *parse_unsigned("12x").unwrap_err().kind(),
            IntErrorKind::InvalidDigit
        );
    }

    #[test]
    fn parse_unsigned_rejects_empty_or_prefix_only_input() {
        assert_eq!(*parse_unsigned("").unwrap_err().kind(), IntErrorKind::Empty);
        assert_eq!(
            *parse_unsigned("0x").unwrap_err().kind(),
            IntErrorKind::Empty
        );
        assert_eq!(
            *parse_unsigned("0o").unwrap_err().kind(),
            IntErrorKind::Empty
        );
        assert_eq!(
            *parse_unsigned("0b").unwrap_err().kind(),
            IntErrorKind::Empty
        );
    }

    #[test]
    fn parse_unsigned_rejects_overflow() {
        assert_eq!(
            *parse_unsigned("18446744073709551616").unwrap_err().kind(),
            IntErrorKind::PosOverflow
        );
        assert_eq!(
            *parse_unsigned("0x10000000000000000").unwrap_err().kind(),
            IntErrorKind::PosOverflow
        );
        assert_eq!(
            *parse_unsigned("0o2000000000000000000000")
                .unwrap_err()
                .kind(),
            IntErrorKind::PosOverflow
        );
        assert_eq!(
            *parse_unsigned("0b10000000000000000000000000000000000000000000000000000000000000000")
                .unwrap_err()
                .kind(),
            IntErrorKind::PosOverflow
        );
    }

    #[test]
    fn parse_expr_accepts_explicit_operands_for_all_operators() {
        let model = BitTwiddleModel::default();

        assert_eq!(parse_expr(&model, "1 + 2"), Ok((1, 2, "+".into())));
        assert_eq!(parse_expr(&model, "3 - 4"), Ok((3, 4, "-".into())));
        assert_eq!(parse_expr(&model, "5 * 6"), Ok((5, 6, "*".into())));
        assert_eq!(parse_expr(&model, "7 / 8"), Ok((7, 8, "/".into())));
        assert_eq!(parse_expr(&model, "9 << 10"), Ok((9, 10, "<<".into())));
        assert_eq!(parse_expr(&model, "11 >> 12"), Ok((11, 12, ">>".into())));
        assert_eq!(parse_expr(&model, "13 | 14"), Ok((13, 14, "|".into())));
        assert_eq!(parse_expr(&model, "15 & 16"), Ok((15, 16, "&".into())));
        assert_eq!(parse_expr(&model, "17 ^ 18"), Ok((17, 18, "^".into())));
    }

    #[test]
    fn parse_expr_accepts_supported_literal_bases() {
        let model = BitTwiddleModel::default();

        assert_eq!(parse_expr(&model, "0xff + 0X10"), Ok((255, 16, "+".into())));
        assert_eq!(parse_expr(&model, "0o17 + 0O10"), Ok((15, 8, "+".into())));
        assert_eq!(parse_expr(&model, "0b1010 + 0B11"), Ok((10, 3, "+".into())));
    }

    #[test]
    fn parse_expr_accepts_surrounding_whitespace() {
        let model = BitTwiddleModel::default();

        assert_eq!(
            parse_expr(&model, " \t 12 \n + \r 34 \t "),
            Ok((12, 34, "+".into()))
        );
    }

    #[test]
    fn parse_expr_uses_model_value_when_left_operand_is_omitted() {
        let model = model_with_value(42);

        assert_eq!(parse_expr(&model, "+ 5"), Ok((42, 5, "+".into())));
        assert_eq!(parse_expr(&model, "<< 2"), Ok((42, 2, "<<".into())));
        assert_eq!(parse_expr(&model, " & 0xff"), Ok((42, 255, "&".into())));
    }

    #[test]
    fn parse_expr_rejects_malformed_expressions() {
        let model = BitTwiddleModel::default();

        assert_eq!(
            parse_expr(&model, "").unwrap_err(),
            "Malformed expression ''"
        );
        assert_eq!(
            parse_expr(&model, "1 +").unwrap_err(),
            "Malformed expression '1 +'"
        );
        assert_eq!(
            parse_expr(&model, "1 2 + 3").unwrap_err(),
            "Malformed expression '1 2 + 3'"
        );
        assert_eq!(
            parse_expr(&model, "1 % 2").unwrap_err(),
            "Malformed expression '1 % 2'"
        );
        assert_eq!(
            parse_expr(&model, "0xg + 1").unwrap_err(),
            "Malformed expression '0xg + 1'"
        );
    }

    #[test]
    fn parse_expr_reports_operand_overflow() {
        let model = BitTwiddleModel::default();

        assert_eq!(
            parse_expr(&model, "18446744073709551616 + 1").unwrap_err(),
            "Error parsing operand 1 - number too large to fit in target type"
        );
        assert_eq!(
            parse_expr(&model, "1 + 18446744073709551616").unwrap_err(),
            "Error parsing operand 2 - number too large to fit in target type"
        );
    }
}
