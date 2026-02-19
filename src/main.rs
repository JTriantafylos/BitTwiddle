use regex::Regex;

use ratatui::{
    Frame,
    crossterm::event::{self, Event, KeyCode, KeyEvent},
    layout::{Constraint, Layout, Position},
    style::{Color, Style},
    text::Text,
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

fn parse_expr(input: &str) -> Result<(u64, u64, String), String> {
    let re = Regex::new(concat!(
        r"^",                                                          // Beginning marker
        r"\s*", // Match any preceding whitespace
        r"(?<operand1>0[xX][0-9a-fA-F]+|0[oO][0-7]+|0[bB][0-1]+|\d+)", // Capture the first operand
        r"\s*", // Match any whitespace
        r"(?<operator>\+|-|\*|\/|<<|>>+)", // Capture the operator
        r"\s*", // Match any whitespace
        r"(?<operand2>0[xX][0-9a-fA-F]+|0[oO][0-7]+|0[bB][0-1]+|\d+)", // Capture the second operand
        r"\s*", // Match any trailing whitespace
        r"$"    // End marker
    ))
    .unwrap();
    let captures = re.captures(input);
    match captures {
        Some(caps) => {
            let operand1: u64 = parse_unsigned(&caps["operand1"])
                .map_err(|e| format!("Error parsing operand 1 - {}", e))?;
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
        "+" => Ok(operand1 + operand2),
        "-" => Ok(operand1 - operand2),
        "*" => Ok(operand1 * operand2),
        "/" => {
            if operand2 == 0 {
                Err("division by zero".into())
            } else {
                Ok(operand1 / operand2)
            }
        }
        "<<" => Ok(operand1 << operand2),
        ">>" => Ok(operand1 >> operand2),
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

#[derive(Default, PartialEq, Eq)]
enum InputMode {
    BitManipulation,
    #[default]
    ExpressionInput,
    HistorySelect,
}

#[derive(Default)]
struct BitTwiddleModel {
    running_state: RunningState,
    val: Option<Result<u64, String>>,
    display_mode: DisplayMode,
    input_mode: InputMode,
    expression_input_text: String,
    expression_input_cursor_index: usize,
}

enum Message {
    KeyPressed(KeyEvent),
    SubmitExpression,
    SwitchInputMode,
    Quit,
}

fn update(model: &mut BitTwiddleModel, msg: Message) -> Option<Message> {
    match msg {
        Message::KeyPressed(key) => match key.code {
            KeyCode::Enter => Some(Message::SubmitExpression),
            KeyCode::Left => todo!(),
            KeyCode::Right => todo!(),
            KeyCode::Home => todo!(),
            KeyCode::End => todo!(),
            KeyCode::Up => todo!(),
            KeyCode::Down => todo!(),
            KeyCode::Backspace => todo!(),
            KeyCode::Char(c) => {
                model.expression_input_text.push(c);
                model.expression_input_cursor_index += 1;
                None
            }
            _ => None,
        },
        Message::SubmitExpression => {
            model.val = match parse_expr(&model.expression_input_text) {
                Ok((operand1, operand2, operator)) => {
                    match compute(operand1, operand2, &operator) {
                        Ok(result) => Some(Ok(result)),
                        Err(e) => Some(Err(format!("Computation error: {}", e))),
                    }
                }
                Err(e) => Some(Err(format!("Parse error: {}", e))),
            };
            model.expression_input_cursor_index = 0;
            model.expression_input_text.clear();
            None
        }
        Message::SwitchInputMode => {
            match model.input_mode {
                InputMode::BitManipulation => model.input_mode = InputMode::ExpressionInput,
                InputMode::ExpressionInput => model.input_mode = InputMode::HistorySelect,
                InputMode::HistorySelect => model.input_mode = InputMode::BitManipulation,
            };
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
        Constraint::Length(3),
        Constraint::Min(1),
    ]);
    let [value_area, input_area, expression_history_area] = vertical.areas(frame.area());

    let val_text = Text::from(match &model.val {
        Some(val) => match val {
            Ok(val) => val.to_string(),
            Err(err) => err.to_string(),
        },
        None => "".to_string(),
    });
    frame.render_widget(
        Paragraph::new(val_text).block(Block::bordered()),
        value_area,
    );

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

    let expression_history = List::new(vec![ListItem::new("Fix expression history!")]).block(
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
