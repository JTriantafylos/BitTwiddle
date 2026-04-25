use ratatui::{
    Frame,
    crossterm::event::{self, Event, KeyCode, KeyEvent},
    layout::{Constraint, Layout, Position},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, List, ListItem, Paragraph},
};

mod expression;

use expression::{Operator, compute, parse_expr};

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
    calculation_history: Vec<(u64, Operator, u64, u64)>,
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

impl BitTwiddleModel {
    fn clear_input(&mut self) -> Option<Message> {
        self.value = 0;
        self.error = None;
        None
    }

    fn submit_expression(&mut self) -> Option<Message> {
        match parse_expr(self.value, &self.expression_input_text) {
            Ok(expression) => {
                match compute(
                    expression.operand1,
                    expression.operand2,
                    expression.operator,
                ) {
                    Ok(result) => {
                        self.value = result;
                        self.error = None;
                        self.expression_input_history
                            .push(self.expression_input_text.clone());
                        self.calculation_history.push((
                            expression.operand1,
                            expression.operator,
                            expression.operand2,
                            result,
                        ));
                    }
                    Err(e) => {
                        self.error = Some(format!("Computation error: {}", e));
                    }
                }
            }
            Err(e) => {
                self.error = Some(format!("Parse error: {}", e));
            }
        };
        self.reset_expression_input();
        None
    }

    fn reset_expression_input(&mut self) {
        self.expression_input_cursor_index = 0;
        self.expression_input_text.clear();
        self.expression_input_history_index = None;
    }

    fn shift_input_cursor(&mut self, offset: isize) -> Option<Message> {
        self.expression_input_cursor_index = self
            .expression_input_cursor_index
            .saturating_add_signed(offset)
            .clamp(0, self.expression_input_text.len());
        None
    }

    fn insert_input_char(&mut self, c: char) -> Option<Message> {
        self.expression_input_text
            .insert(self.expression_input_cursor_index, c);
        Some(Message::ShiftInputCursor(1))
    }

    fn backspace_input_char(&mut self) -> Option<Message> {
        if !self.expression_input_text.is_empty() && self.expression_input_cursor_index > 0 {
            self.expression_input_text
                .remove(self.expression_input_cursor_index - 1);
            Some(Message::ShiftInputCursor(-1))
        } else {
            None
        }
    }

    fn delete_input_char(&mut self) -> Option<Message> {
        if !self.expression_input_text.is_empty()
            && self.expression_input_cursor_index < self.expression_input_text.len()
        {
            self.expression_input_text
                .remove(self.expression_input_cursor_index);
        }
        None
    }

    fn shift_selected_bit(&mut self, offset: isize) -> Option<Message> {
        self.selected_bit_index = self
            .selected_bit_index
            .saturating_add_signed(offset)
            .clamp(0, 63);
        None
    }

    fn toggle_selected_bit(&mut self) -> Option<Message> {
        self.value ^= 1_u64 << self.selected_bit_index;
        self.error = None;
        None
    }

    fn prev_input_mode(&mut self) -> Option<Message> {
        self.input_mode = self.input_mode.prev();
        None
    }

    fn next_input_mode(&mut self) -> Option<Message> {
        self.input_mode = self.input_mode.next();
        None
    }

    fn prev_display_mode(&mut self) -> Option<Message> {
        self.display_mode = self.display_mode.prev();
        None
    }

    fn next_display_mode(&mut self) -> Option<Message> {
        self.display_mode = self.display_mode.next();
        None
    }

    fn dismiss_error(&mut self) -> Option<Message> {
        self.error = None;
        None
    }

    fn quit(&mut self) -> Option<Message> {
        self.running_state = RunningState::Done;
        None
    }

    fn prev_input_history(&mut self) -> Option<Message> {
        match self.expression_input_history_index {
            Some(0) => {
                self.expression_input_text = String::default();
                self.expression_input_cursor_index = 0;
                self.expression_input_history_index = None;
            }
            Some(n) if !self.expression_input_history.is_empty() => {
                let index = n
                    .saturating_sub(1)
                    .clamp(0, self.expression_input_history.len() - 1);
                self.load_input_history(index);
            }
            _ => {}
        };
        None
    }

    fn next_input_history(&mut self) -> Option<Message> {
        if !self.expression_input_history.is_empty() {
            let index = match self.expression_input_history_index {
                None => 0,
                Some(n) => n
                    .saturating_add(1)
                    .clamp(0, self.expression_input_history.len() - 1),
            };
            self.load_input_history(index);
        }
        None
    }

    fn load_input_history(&mut self, index: usize) {
        self.expression_input_text = self
            .expression_input_history
            .iter()
            .rev()
            .nth(index)
            .unwrap()
            .clone();
        self.expression_input_cursor_index = self.expression_input_text.len();
        self.expression_input_history_index = Some(index);
    }
}

enum Message {
    KeyPressed(KeyEvent),
    ClearInput,
    SubmitExpression,
    ShiftInputCursor(isize),
    InsertInputChar(char),
    BackspaceInputChar,
    DeleteInputChar,
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

fn map_key_to_message(model: &BitTwiddleModel, key: KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::BackTab => Some(Message::PrevInputMode),
        KeyCode::Tab => Some(Message::NextInputMode),
        KeyCode::Char('M') => Some(Message::PrevDisplayMode),
        KeyCode::Char('m') => Some(Message::NextDisplayMode),
        KeyCode::Char('c') => Some(Message::ClearInput),
        KeyCode::Esc => Some(Message::DismissError),
        _ => map_mode_key_to_message(&model.input_mode, key),
    }
}

fn map_mode_key_to_message(input_mode: &InputMode, key: KeyEvent) -> Option<Message> {
    match input_mode {
        InputMode::BitManipulation => map_bit_manipulation_key_to_message(key),
        InputMode::ExpressionInput => map_expression_input_key_to_message(key),
        InputMode::HistorySelect => None,
    }
}

fn map_bit_manipulation_key_to_message(key: KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Left => Some(Message::ShiftSelectedBit(1)),
        KeyCode::Right => Some(Message::ShiftSelectedBit(-1)),
        KeyCode::Up => Some(Message::ShiftSelectedBit(32)),
        KeyCode::Down => Some(Message::ShiftSelectedBit(-32)),
        KeyCode::Char(' ') => Some(Message::ToggleSelectedBit),
        _ => None,
    }
}

fn map_expression_input_key_to_message(key: KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Enter => Some(Message::SubmitExpression),
        KeyCode::Left => Some(Message::ShiftInputCursor(-1)),
        KeyCode::Right => Some(Message::ShiftInputCursor(1)),
        KeyCode::Home => Some(Message::ShiftInputCursor(isize::MIN)),
        KeyCode::End => Some(Message::ShiftInputCursor(isize::MAX)),
        KeyCode::Up => Some(Message::NextInputHistory),
        KeyCode::Down => Some(Message::PrevInputHistory),
        KeyCode::Backspace => Some(Message::BackspaceInputChar),
        KeyCode::Delete => Some(Message::DeleteInputChar),
        KeyCode::Char(c) => Some(Message::InsertInputChar(c)),
        _ => None,
    }
}

fn update(model: &mut BitTwiddleModel, msg: Message) -> Option<Message> {
    match msg {
        Message::KeyPressed(key) => map_key_to_message(model, key),
        Message::ClearInput => model.clear_input(),
        Message::SubmitExpression => model.submit_expression(),
        Message::ShiftInputCursor(offset) => model.shift_input_cursor(offset),
        Message::InsertInputChar(c) => model.insert_input_char(c),
        Message::BackspaceInputChar => model.backspace_input_char(),
        Message::DeleteInputChar => model.delete_input_char(),
        Message::ShiftSelectedBit(offset) => model.shift_selected_bit(offset),
        Message::ToggleSelectedBit => model.toggle_selected_bit(),
        Message::PrevInputMode => model.prev_input_mode(),
        Message::NextInputMode => model.next_input_mode(),
        Message::PrevInputHistory => model.prev_input_history(),
        Message::NextInputHistory => model.next_input_history(),
        Message::PrevDisplayMode => model.prev_display_mode(),
        Message::NextDisplayMode => model.next_display_mode(),
        Message::DismissError => model.dismiss_error(),
        Message::Quit => model.quit(),
    }
}

fn view(model: &BitTwiddleModel, frame: &mut Frame) {
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
            terminal.draw(|f| view(&model, f))?;

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
