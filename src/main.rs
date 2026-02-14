use regex::Regex;
use std::process::ExitCode;

use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    layout::{Constraint, Layout, Position},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, List, ListItem, Paragraph},
};

fn main() -> ExitCode {
    let terminal = ratatui::init();
    let app_result = App::new().run(terminal);
    ratatui::restore();
    app_result.unwrap_or(ExitCode::FAILURE)
}

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
        r"\s*",                                                        // Match any preceding whitespace
        r"(?<operand1>0[xX][0-9a-fA-F]+|0[oO][0-7]+|0[bB][0-1]+|\d+)", // Capture the first operand
        r"\s*",                                                        // Match any whitespace
        r"(?<operator>\+|-|\*|\/|<<|>>+)",                             // Capture the operator
        r"\s*",                                                        // Match any whitespace
        r"(?<operand2>0[xX][0-9a-fA-F]+|0[oO][0-7]+|0[bB][0-1]+|\d+)", // Capture the second operand
        r"\s*",                                                        // Match any trailing whitespace
        r"$"                                                           // End marker
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

/// App holds the state of the application
struct App {
    /// Current value of the input box
    input: String,
    /// Position of cursor in the editor area.
    character_index: usize,
    /// Current input mode
    input_mode: InputMode,
    /// History of previous inputs
    input_history: Vec<String>,
    /// Position within the input history list
    input_history_index: usize,
    /// History of previous calculations
    expression_history: Vec<String>,
}

enum InputMode {
    ExpressionInput,
    BitManipulation,
    HistorySelect,
}

impl App {
    const fn new() -> Self {
        Self {
            input: String::new(),
            input_mode: InputMode::ExpressionInput,
            input_history: Vec::new(),
            input_history_index: 0,
            expression_history: Vec::new(),
            character_index: 0,
        }
    }

    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.input.chars().count())
    }

    fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.character_index.saturating_sub(1);
        self.character_index = self.clamp_cursor(cursor_moved_left);
    }

    fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.character_index.saturating_add(1);
        self.character_index = self.clamp_cursor(cursor_moved_right);
    }

    fn get_previous_input(&mut self) {
        if self.input_history_index == self.input_history.len() {
            return;
        }

        self.input_history_index = self.input_history_index.saturating_add(1);
        self.input = self.input_history[self.input_history_index - 1].clone();
        self.character_index = self.clamp_cursor(self.input.len());
    }

    fn get_next_input(&mut self) {
        match self.input_history_index {
            0 => return,
            1 => {
                self.input_history_index = self.input_history_index.saturating_sub(1);
                self.input.clear();
                self.character_index = self.clamp_cursor(0);
            }
            _ => {
                self.input_history_index = self.input_history_index.saturating_sub(1);
                self.input = self.input_history[self.input_history_index - 1].clone();
                self.character_index = self.clamp_cursor(self.input.len());
            }
        }
    }

    fn enter_char(&mut self, new_char: char) {
        let index = self.byte_index();
        self.input.insert(index, new_char);
        self.move_cursor_right();
    }

    /// Returns the byte index based on the character position.
    ///
    /// Since each character in a string can be contain multiple bytes, it's necessary to calculate
    /// the byte index based on the index of the character.
    fn byte_index(&self) -> usize {
        self.input
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.character_index)
            .unwrap_or(self.input.len())
    }

    fn delete_char(&mut self) {
        let is_not_cursor_leftmost = self.character_index != 0;
        if is_not_cursor_leftmost {
            // Method "remove" is not used on the saved text for deleting the selected char.
            // Reason: Using remove on String works on bytes instead of the chars.
            // Using remove would require special care because of char boundaries.

            let current_index = self.character_index;
            let from_left_to_current_index = current_index - 1;

            // Getting all characters before the selected character.
            let before_char_to_delete = self.input.chars().take(from_left_to_current_index);
            // Getting all characters after selected character.
            let after_char_to_delete = self.input.chars().skip(current_index);

            // Put all characters together except the selected one.
            // By leaving the selected one out, it is forgotten and therefore deleted.
            self.input = before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left();
        }
    }

    fn reset_cursor(&mut self) {
        self.character_index = 0;
    }

    fn submit_expression(&mut self) {
        self.input_history_index = 0;
        self.input_history.insert(0, self.input.clone());
        match parse_expr(&self.input) {
            Ok((operand1, operand2, operator)) => match compute(operand1, operand2, &operator) {
                Ok(result) => self.expression_history.push(format!(
                    "{} {} {} = {}",
                    operand1, operator, operand2, result
                )),
                Err(e) => self
                    .expression_history
                    .push(format!("Computation error: {}", e)),
            },
            Err(e) => self.expression_history.push(format!("Parse error: {}", e)),
        }
        self.input.clear();
        self.reset_cursor();
    }

    fn run(mut self, mut terminal: DefaultTerminal) -> Result<ExitCode, std::io::Error> {
        loop {
            terminal.draw(|frame| self.draw(frame))?;

            if let Event::Key(key) = event::read()? {
                match self.input_mode {
                    InputMode::BitManipulation => match key.code {
                        KeyCode::Tab => self.input_mode = InputMode::ExpressionInput,
                        KeyCode::Char('q') => return Ok(ExitCode::SUCCESS),
                        _ => {}
                    },
                    InputMode::ExpressionInput => match key.code
                    {
                        KeyCode::Tab => self.input_mode = InputMode::HistorySelect,
                        KeyCode::Char('q') => return Ok(ExitCode::SUCCESS),
                        KeyCode::Enter => self.submit_expression(),
                        KeyCode::Backspace => self.delete_char(),
                        KeyCode::Left => self.move_cursor_left(),
                        KeyCode::Right => self.move_cursor_right(),
                        KeyCode::Home => self.character_index = self.clamp_cursor(0),
                        KeyCode::End => self.character_index = self.clamp_cursor(usize::MAX),
                        KeyCode::Up => self.get_previous_input(),
                        KeyCode::Down => self.get_next_input(),
                        KeyCode::Char(to_insert) => self.enter_char(to_insert),
                        _ => {}
                    },
                    InputMode::HistorySelect => match key.code {
                        KeyCode::Tab => self.input_mode = InputMode::BitManipulation,
                        KeyCode::Char('q') => return Ok(ExitCode::SUCCESS),
                        _ => {}
                    }
                }
            }
        }
    }

    fn draw(&self, frame: &mut Frame) {
        let vertical = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Min(1),
        ]);
        let [help_area, input_area, expression_history_area] = vertical.areas(frame.area());

        let text = Text::from("FIXME: Add keybind help");
        let help_message = Paragraph::new(text);
        frame.render_widget(help_message, help_area);

        let input = Paragraph::new(self.input.as_str()).block(
            Block::bordered()
                .border_style(match self.input_mode {
                    InputMode::ExpressionInput => Style::default().fg(Color::Yellow),
                    _ => Style::default(),
                })
                .title("Input"),
        );
        frame.render_widget(input, input_area);
        match self.input_mode {
            InputMode::ExpressionInput => frame.set_cursor_position(Position::new(
                // Draw the cursor at the current position in the input field.
                // This position is can be controlled via the left and right arrow key
                input_area.x + self.character_index as u16 + 1,
                // Move one line down, from the border to the input line
                input_area.y + 1,
            )),
            _ => {}
        }

        let expression_history: Vec<ListItem> = self
            .expression_history
            .iter()
            .rev()
            .map(|m| {
                let content = Line::from(Span::raw(m));
                ListItem::new(content)
            })
            .collect();
        let expression_history = List::new(expression_history).block(
            Block::bordered()
                .border_style(match self.input_mode {
                    InputMode::HistorySelect => Style::default().fg(Color::Yellow),
                    _ => Style::default(),
                })
                .title("History"),
        );
        frame.render_widget(expression_history, expression_history_area);
    }
}
