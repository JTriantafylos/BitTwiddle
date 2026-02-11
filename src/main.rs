use regex::Regex;
use std::process::ExitCode;

use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    layout::{Constraint, Layout, Position},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, List, ListItem, Paragraph},
};

fn main() -> ExitCode {
    let terminal = ratatui::init();
    let app_result = App::new().run(terminal);
    ratatui::restore();
    if app_result.is_ok() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn parse_unsigned(s: &str) -> Result<u64, std::num::ParseIntError> {
    let s = s.trim_start();
    if let Some(rest) = s.strip_prefix("0x") {
        u64::from_str_radix(rest, 16)
    } else if let Some(rest) = s.strip_prefix("0o") {
        u64::from_str_radix(rest, 8)
    } else if let Some(rest) = s.strip_prefix("0b") {
        u64::from_str_radix(rest, 2)
    } else {
        u64::from_str_radix(s, 10)
    }
}

fn parse_expr(input: &str) -> Result<(u64, u64, String), String> {
    let re = Regex::new(concat!(
        r"^",                                  // Beginning marker
        r"\s*",                                // Match any preceding whitespace
        r"(?<operand1>0[xX][0-9a-fA-F]+|\d+)", // Capture the first operand
        r"\s*",                                // Match any whitespace
        r"(?<operator>\+|-|\*|\/|<<|>>+)",     // Capture the operator
        r"\s*",                                // Match any whitespace
        r"(?<operand2>0[xX][0-9a-fA-F]+|\d+)", // Capture the second operand
        r"\s*",                                // Match any trailing whitespace
        r"$"                                   // End marker
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
    /// History of previous calculations
    history: Vec<String>,
}

enum InputMode {
    Normal,
    Editing,
}

impl App {
    const fn new() -> Self {
        Self {
            input: String::new(),
            input_mode: InputMode::Normal,
            history: Vec::new(),
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
        match parse_expr(&self.input) {
            Ok((operand1, operand2, operator)) => match compute(operand1, operand2, &operator) {
                Ok(result) => self.history.push(format!(
                    "{} {} {} = {}",
                    operand1, operator, operand2, result
                )),
                Err(e) => self.history.push(format!("Computation error: {}", e)),
            },
            Err(e) => self.history.push(format!("Parse error: {}", e)),
        }
        self.input.clear();
        self.reset_cursor();
    }

    fn run(mut self, mut terminal: DefaultTerminal) -> Result<i32, std::io::Error> {
        loop {
            terminal.draw(|frame| self.draw(frame))?;

            if let Event::Key(key) = event::read()? {
                match self.input_mode {
                    InputMode::Normal => match key.code {
                        KeyCode::Char('e') => {
                            self.input_mode = InputMode::Editing;
                        }
                        KeyCode::Char('q') => {
                            return Ok(0);
                        }
                        _ => {}
                    },
                    InputMode::Editing if key.kind == KeyEventKind::Press => match key.code {
                        KeyCode::Enter => self.submit_expression(),
                        KeyCode::Char(to_insert) => self.enter_char(to_insert),
                        KeyCode::Backspace => self.delete_char(),
                        KeyCode::Left => self.move_cursor_left(),
                        KeyCode::Right => self.move_cursor_right(),
                        KeyCode::Esc => self.input_mode = InputMode::Normal,
                        _ => {}
                    },
                    InputMode::Editing => {}
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
        let [help_area, input_area, history_area] = vertical.areas(frame.area());

        let (msg, style) = match self.input_mode {
            InputMode::Normal => (
                vec![
                    "Press ".into(),
                    "q".bold(),
                    " to exit, ".into(),
                    "e".bold(),
                    " to start editing.".bold(),
                ],
                Style::default().add_modifier(Modifier::RAPID_BLINK),
            ),
            InputMode::Editing => (
                vec![
                    "Press ".into(),
                    "Esc".bold(),
                    " to stop editing, ".into(),
                    "Enter".bold(),
                    " to submit".into(),
                ],
                Style::default(),
            ),
        };
        let text = Text::from(Line::from(msg)).patch_style(style);
        let help_message = Paragraph::new(text);
        frame.render_widget(help_message, help_area);

        let input = Paragraph::new(self.input.as_str())
            .style(match self.input_mode {
                InputMode::Normal => Style::default(),
                InputMode::Editing => Style::default().fg(Color::Yellow),
            })
            .block(Block::bordered().title("Input"));
        frame.render_widget(input, input_area);
        match self.input_mode {
            // Hide the cursor. `Frame` does this by default, so we don't need to do anything here
            InputMode::Normal => {}

            // Make the cursor visible and ask ratatui to put it at the specified coordinates after
            // rendering
            InputMode::Editing => frame.set_cursor_position(Position::new(
                // Draw the cursor at the current position in the input field.
                // This position is can be controlled via the left and right arrow key
                input_area.x + self.character_index as u16 + 1,
                // Move one line down, from the border to the input line
                input_area.y + 1,
            )),
        }

        let history: Vec<ListItem> = self
            .history
            .iter()
            .map(|m| {
                let content = Line::from(Span::raw(m));
                ListItem::new(content)
            })
            .collect();
        let history = List::new(history).block(Block::bordered().title("History"));
        frame.render_widget(history, history_area);
    }
}
