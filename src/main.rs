use ratatui::{
    Frame,
    crossterm::event::{self, Event, KeyCode, KeyEvent},
    layout::{Constraint, Layout, Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Clear, List, ListItem, Paragraph},
};

mod expression;
mod model;

use model::{BitTwiddleModel, DisplayMode, InputMode, RunningState};

#[derive(Debug, PartialEq, Eq)]
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
    PrevCalculationHistory,
    NextCalculationHistory,
    CopySelectedCalculationHistory,
    PrevDisplayMode,
    NextDisplayMode,
    DismissError,
    RequestQuit,
    CancelQuit,
    ConfirmQuit,
}

fn map_key_to_message(model: &BitTwiddleModel, key: KeyEvent) -> Option<Message> {
    if model.show_quit_confirmation {
        return match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => Some(Message::ConfirmQuit),
            KeyCode::Char('n') | KeyCode::Char('N') => Some(Message::CancelQuit),
            _ => None,
        };
    }

    match key.code {
        KeyCode::BackTab => Some(Message::PrevInputMode),
        KeyCode::Tab => Some(Message::NextInputMode),
        KeyCode::Char('M') => Some(Message::PrevDisplayMode),
        KeyCode::Char('m') => Some(Message::NextDisplayMode),
        KeyCode::Char('c') => Some(Message::ClearInput),
        KeyCode::Char('q') => Some(Message::RequestQuit),
        KeyCode::Esc => Some(Message::DismissError),
        _ => map_mode_key_to_message(&model.input_mode, key),
    }
}

fn map_mode_key_to_message(input_mode: &InputMode, key: KeyEvent) -> Option<Message> {
    match input_mode {
        InputMode::BitManipulation => map_bit_manipulation_key_to_message(key),
        InputMode::ExpressionInput => map_expression_input_key_to_message(key),
        InputMode::HistorySelect => map_history_select_key_to_message(key),
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

fn map_history_select_key_to_message(key: KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Up => Some(Message::PrevCalculationHistory),
        KeyCode::Down => Some(Message::NextCalculationHistory),
        KeyCode::Enter => Some(Message::CopySelectedCalculationHistory),
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
        Message::PrevCalculationHistory => model.prev_calculation_history(),
        Message::NextCalculationHistory => model.next_calculation_history(),
        Message::CopySelectedCalculationHistory => model.copy_selected_calculation_history(),
        Message::PrevDisplayMode => model.prev_display_mode(),
        Message::NextDisplayMode => model.next_display_mode(),
        Message::DismissError => model.dismiss_error(),
        Message::RequestQuit => model.request_quit(),
        Message::CancelQuit => model.cancel_quit(),
        Message::ConfirmQuit => model.confirm_quit(),
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

    let mut bit_visualization_block = Block::bordered()
        .border_style(match model.input_mode {
            InputMode::BitManipulation => Style::default().fg(Color::Yellow),
            _ => Style::default(),
        })
        .title("Bits");
    if bit_manipulation_mode {
        bit_visualization_block =
            bit_visualization_block.title_bottom(format!(" Bit {} ", selected_bit_index));
    }

    let bit_visualization = Paragraph::new(bit_visualization_text)
        .block(bit_visualization_block)
        .centered();
    frame.render_widget(bit_visualization, bit_area);

    let expression_history = List::new(model.calculation_history.iter().rev().enumerate().map(
        |(index, e)| {
            let item = ListItem::new(model.format_calculation_history_entry(e));
            if model.input_mode == InputMode::HistorySelect
                && model.calculation_history_selection_index == Some(index)
            {
                item.style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::REVERSED),
                )
            } else {
                item
            }
        },
    ))
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
        [
            "[c] Clear",
            "[m] Cycle display mode (forward)",
            "[M] Cycle display mode (backward)",
            "[arrows] Select bit",
            "[space] Toggle bit",
        ]
        .join(" "),
    );
    frame.render_widget(Paragraph::new(help_text).centered(), help_area);

    if model.show_quit_confirmation {
        render_quit_confirmation(frame);
    }
}

fn render_quit_confirmation(frame: &mut Frame) {
    let area = centered_rect(44, 5, frame.area());
    let dialog = Paragraph::new(Text::from(vec![
        Line::from("Quit BitTwiddle?"),
        Line::from(""),
        Line::from("Press Y to quit or N to continue."),
    ]))
    .block(
        Block::bordered()
            .title("Confirm Quit")
            .border_style(Style::default().fg(Color::Yellow)),
    )
    .centered();

    frame.render_widget(Clear, area);
    frame.render_widget(dialog, area);
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);

    Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    }
}

fn handle_event() -> Result<Option<Message>, std::io::Error> {
    if let Event::Key(key) = event::read()? {
        Ok(Some(Message::KeyPressed(key)))
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

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::crossterm::event::KeyModifiers;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn q_requests_quit_confirmation() {
        let model = BitTwiddleModel::default();

        assert_eq!(
            map_key_to_message(&model, key(KeyCode::Char('q'))),
            Some(Message::RequestQuit)
        );
    }

    #[test]
    fn quit_confirmation_accepts_y_or_n_only() {
        let model = BitTwiddleModel {
            show_quit_confirmation: true,
            ..Default::default()
        };

        assert_eq!(
            map_key_to_message(&model, key(KeyCode::Char('y'))),
            Some(Message::ConfirmQuit)
        );
        assert_eq!(
            map_key_to_message(&model, key(KeyCode::Char('Y'))),
            Some(Message::ConfirmQuit)
        );
        assert_eq!(
            map_key_to_message(&model, key(KeyCode::Char('n'))),
            Some(Message::CancelQuit)
        );
        assert_eq!(
            map_key_to_message(&model, key(KeyCode::Char('N'))),
            Some(Message::CancelQuit)
        );
        assert_eq!(map_key_to_message(&model, key(KeyCode::Char('q'))), None);
    }

    #[test]
    fn history_select_maps_up_down_and_enter() {
        let model = BitTwiddleModel {
            input_mode: InputMode::HistorySelect,
            ..Default::default()
        };

        assert_eq!(
            map_key_to_message(&model, key(KeyCode::Up)),
            Some(Message::PrevCalculationHistory)
        );
        assert_eq!(
            map_key_to_message(&model, key(KeyCode::Down)),
            Some(Message::NextCalculationHistory)
        );
        assert_eq!(
            map_key_to_message(&model, key(KeyCode::Enter)),
            Some(Message::CopySelectedCalculationHistory)
        );
    }
}
