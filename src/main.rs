use std::ops::{BitAnd, BitOr, BitXor};

use regex::Regex;

use ratatui::{
    Frame,
    crossterm::event::{self, Event, KeyCode, KeyEvent},
    layout::{Constraint, Layout, Position},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, List, ListItem, Paragraph},
};

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

fn parse_expr(model: &BitTwiddleModel, input: &str) -> Result<(u64, u64, String), String> {
    let re = Regex::new(concat!(
        r"^",                                                           // Beginning marker
        r"\s*", // Match any preceding whitespace
        r"(?<operand1>0[xX][0-9a-fA-F]+|0[oO][0-7]+|0[bB][0-1]+|\d+)?", // Capture the first operand
        r"\s*", // Match any whitespace
        r"(?<operator>\+|-|\*|\/|<<|>>|\||\&|\^+)", // Capture the operator
        r"\s*", // Match any whitespace
        r"(?<operand2>0[xX][0-9a-fA-F]+|0[oO][0-7]+|0[bB][0-1]+|\d+)", // Capture the second operand
        r"\s*", // Match any trailing whitespace
        r"$"    // End marker
    ))
    .unwrap();
    let captures = re.captures(input);
    match captures {
        Some(caps) => {
            let operand1_match = caps.name("operand1");
            let operand1: u64 = match operand1_match {
                Some(s) => parse_unsigned(s.as_str())
                    .map_err(|e| format!("Error parsing operand 1 - {}", e))?,
                None => *model
                    .val
                    .as_ref()
                    .map_err(|_e| format!("Malformed expression '{}'", input))?,
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
        "<<" => operand1
            .checked_shl(operand2 as u32)
            .ok_or(format!("Invalid left-shift: {}", operand2)),
        ">>" => operand1
            .checked_shr(operand2 as u32)
            .ok_or(format!("Invalid right-shift: {}", operand2)),
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
    val: Result<u64, String>,
    display_mode: DisplayMode,
    input_mode: InputMode,
    expression_input_text: String,
    expression_input_cursor_index: usize,
    expression_input_history: Vec<String>,
    expression_input_history_index: Option<usize>,
    calculation_history: Vec<(u64, String, u64, u64)>,
}

impl Default for BitTwiddleModel {
    fn default() -> Self {
        Self {
            running_state: Default::default(),
            val: Ok(0),
            display_mode: Default::default(),
            input_mode: Default::default(),
            expression_input_text: Default::default(),
            expression_input_cursor_index: Default::default(),
            expression_input_history: Vec::default(),
            expression_input_history_index: None,
            calculation_history: Vec::default(),
        }
    }
}

enum Message {
    KeyPressed(KeyEvent),
    ClearInput,
    SubmitExpression,
    ShiftInputCursor(isize),
    PrevInputMode,
    NextInputMode,
    PrevInputHistory,
    NextInputHistory,
    PrevDisplayMode,
    NextDisplayMode,
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
                    model.expression_input_text.remove(model.expression_input_cursor_index - 1);
                    Some(Message::ShiftInputCursor(-1))
                } else {
                    None
                }
            }
            KeyCode::Delete if model.input_mode == InputMode::ExpressionInput => {
                if !model.expression_input_text.is_empty()
                    && model.expression_input_cursor_index < model.expression_input_text.len()
                {
                    model.expression_input_text.remove(model.expression_input_cursor_index);
                }
                None
            }
            KeyCode::Char(c) if model.input_mode == InputMode::ExpressionInput => {
                model.expression_input_text.insert(model.expression_input_cursor_index, c);
                Some(Message::ShiftInputCursor(1))
            }

            _ => None,
        },
        Message::ClearInput => {
            model.val = Ok(0);
            None
        }
        Message::SubmitExpression => {
            model.val = match parse_expr(model, &model.expression_input_text) {
                Ok((operand1, operand2, operator)) => {
                    match compute(operand1, operand2, &operator) {
                        Ok(result) => {
                            model
                                .expression_input_history
                                .push(model.expression_input_text.clone());
                            model
                                .calculation_history
                                .push((operand1, operator, operand2, result));
                            Ok(result)
                        }
                        Err(e) => Err(format!("Computation error: {}", e)),
                    }
                }
                Err(e) => Err(format!("Parse error: {}", e)),
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
    ]);
    let [value_areas, bit_area, input_area, expression_history_area] = vertical.areas(frame.area());

    let [value_area, mode_area] =
        Layout::horizontal([Constraint::Min(1), Constraint::Length(5)]).areas(value_areas);

    let val_text = Text::from(match &model.val {
        Ok(val) => match model.display_mode {
            DisplayMode::Decimal => format!("{val}"),
            DisplayMode::Hex => format!("{val:#X}"),
            DisplayMode::Octal => format!("{val:#o}"),
        },
        Err(err) => err.to_string(),
    });
    frame.render_widget(
        Paragraph::new(val_text).block(Block::bordered().border_style(match model.input_mode {
            InputMode::BitManipulation => Style::default().fg(Color::Yellow),
            _ => Style::default(),
        })),
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
    let display_value = model.val.as_ref().unwrap_or(&0);
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
                            Span::raw(
                                (0..4)
                                    .rev()
                                    .map(|nibble_num| {
                                        (0..4)
                                            .rev()
                                            .map(|nibble_index| {
                                                if (display_value
                                                    >> (((line_num * BITS_PER_LINE)
                                                        + (word_num * BITS_PER_WORD)
                                                        + (nibble_num * BITS_PER_NIBBLE))
                                                        + nibble_index))
                                                    & 1
                                                    == 1
                                                {
                                                    "1"
                                                } else {
                                                    "0"
                                                }
                                            })
                                            .collect::<Vec<&str>>()
                                            .join(" ")
                                    })
                                    .collect::<Vec<String>>()
                                    .join("  "),
                            ),
                            Span::raw("  "),
                        ]
                    })
                    .collect::<Vec<Span>>(),
            )
        })
        .collect::<Vec<Line>>();

    let bit_visualization = Paragraph::new(bit_visualization_text).block(
        Block::bordered()
            .border_style(match model.input_mode {
                InputMode::BitManipulation => Style::default().fg(Color::Yellow),
                _ => Style::default(),
            })
            .title("Bits"),
    );
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

    ratatui::restore();
    Ok(())
}
