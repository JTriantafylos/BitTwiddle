use ratatui::{
    Frame,
    crossterm::{
        event::{
            self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
            MouseButton, MouseEvent, MouseEventKind,
        },
        execute,
    },
    layout::{Constraint, Layout, Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Clear, List, ListItem, Paragraph},
};
use std::io;

mod expression;
mod model;

use expression::IntegerWidth;
use model::{BitTwiddleModel, DisplayMode, InputMode, RunningState};

#[derive(Debug, PartialEq, Eq)]
enum Message {
    KeyPressed(KeyEvent),
    MousePressed(MouseEvent, Rect),
    ClearInput,
    SubmitExpression,
    ShiftInputCursor(isize),
    InsertInputChar(char),
    BackspaceInputChar,
    DeleteInputChar,
    ShiftSelectedBit(isize),
    ToggleSelectedBit,
    ToggleBit(usize),
    FocusExpressionInput(usize),
    FocusCalculationHistory(Option<usize>),
    PrevInputMode,
    NextInputMode,
    PrevInputHistory,
    NextInputHistory,
    PrevCalculationHistory,
    NextCalculationHistory,
    CopySelectedCalculationHistory,
    PrevDisplayMode,
    NextDisplayMode,
    PrevIntegerWidth,
    NextIntegerWidth,
    DismissError,
    RequestQuit,
    CancelQuit,
    ConfirmQuit,
}

fn map_key_to_message(model: &BitTwiddleModel, key: KeyEvent) -> Option<Message> {
    if model.show_quit_confirmation {
        return match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => Some(Message::ConfirmQuit),
            _ => Some(Message::CancelQuit),
        };
    }

    match key.code {
        KeyCode::BackTab => Some(Message::PrevInputMode),
        KeyCode::Tab => Some(Message::NextInputMode),
        KeyCode::Char('M') => Some(Message::PrevDisplayMode),
        KeyCode::Char('m') => Some(Message::NextDisplayMode),
        KeyCode::Char('W') => Some(Message::PrevIntegerWidth),
        KeyCode::Char('w') => Some(Message::NextIntegerWidth),
        KeyCode::Char('c') if key.modifiers == KeyModifiers::CONTROL => Some(Message::ClearInput),
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
        Message::MousePressed(mouse, area) => map_mouse_to_message(model, mouse, area),
        Message::ClearInput => model.clear_input(),
        Message::SubmitExpression => model.submit_expression(),
        Message::ShiftInputCursor(offset) => model.shift_input_cursor(offset),
        Message::InsertInputChar(c) => model.insert_input_char(c),
        Message::BackspaceInputChar => model.backspace_input_char(),
        Message::DeleteInputChar => model.delete_input_char(),
        Message::ShiftSelectedBit(offset) => model.shift_selected_bit(offset),
        Message::ToggleSelectedBit => model.toggle_selected_bit(),
        Message::ToggleBit(bit_index) => model.toggle_bit(bit_index),
        Message::FocusExpressionInput(cursor_index) => model.focus_expression_input(cursor_index),
        Message::FocusCalculationHistory(selection_index) => {
            model.focus_calculation_history(selection_index)
        }
        Message::PrevInputMode => model.prev_input_mode(),
        Message::NextInputMode => model.next_input_mode(),
        Message::PrevInputHistory => model.prev_input_history(),
        Message::NextInputHistory => model.next_input_history(),
        Message::PrevCalculationHistory => model.prev_calculation_history(),
        Message::NextCalculationHistory => model.next_calculation_history(),
        Message::CopySelectedCalculationHistory => model.copy_selected_calculation_history(),
        Message::PrevDisplayMode => model.prev_display_mode(),
        Message::NextDisplayMode => model.next_display_mode(),
        Message::PrevIntegerWidth => model.prev_integer_width(),
        Message::NextIntegerWidth => model.next_integer_width(),
        Message::DismissError => model.dismiss_error(),
        Message::RequestQuit => model.request_quit(),
        Message::CancelQuit => model.cancel_quit(),
        Message::ConfirmQuit => model.confirm_quit(),
    }
}

fn map_mouse_to_message(
    model: &BitTwiddleModel,
    mouse: MouseEvent,
    frame_area: Rect,
) -> Option<Message> {
    if model.show_quit_confirmation || mouse.kind != MouseEventKind::Down(MouseButton::Left) {
        return None;
    }

    let [_, bit_area, input_area, expression_history_area, _] = app_areas(frame_area);

    hit_test_bit(model.integer_width, bit_area, mouse.column, mouse.row)
        .map(Message::ToggleBit)
        .or_else(|| {
            hit_test_input(input_area, mouse.column, mouse.row).map(Message::FocusExpressionInput)
        })
        .or_else(|| {
            hit_test_history(
                expression_history_area,
                model.calculation_history.len(),
                mouse.column,
                mouse.row,
            )
            .map(Message::FocusCalculationHistory)
        })
}

fn app_areas(area: Rect) -> [Rect; 5] {
    Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(4),
        Constraint::Length(3),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .areas(area)
}

fn view(model: &BitTwiddleModel, frame: &mut Frame) {
    let [
        value_areas,
        bit_area,
        input_area,
        expression_history_area,
        help_area,
    ] = app_areas(frame.area());

    let [value_area, mode_area, width_area] = Layout::horizontal([
        Constraint::Min(1),
        Constraint::Length(7),
        Constraint::Length(14),
    ])
    .areas(value_areas);

    let val_text = Text::from(match &model.error {
        Some(err) => err.to_string(),
        None => model.format_value_readout(),
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
    let signed_indicator = Span::styled(
        "S",
        match model.display_mode {
            DisplayMode::Signed => Style::default().underlined(),
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
        signed_indicator,
        hex_indicator,
        oct_indicator,
    ]))
    .block(Block::bordered());
    frame.render_widget(mode_widget, mode_area);

    let width_indicator = Paragraph::new(Line::from(vec![
        width_span(model.integer_width, IntegerWidth::Eight, "8"),
        Span::raw(" "),
        width_span(model.integer_width, IntegerWidth::Sixteen, "16"),
        Span::raw(" "),
        width_span(model.integer_width, IntegerWidth::ThirtyTwo, "32"),
        Span::raw(" "),
        width_span(model.integer_width, IntegerWidth::SixtyFour, "64"),
    ]))
    .block(Block::bordered());
    frame.render_widget(width_indicator, width_area);

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

    let display_value = model.value;
    let bit_manipulation_mode = model.input_mode == InputMode::BitManipulation;
    let selected_bit_index = model.selected_bit_index;
    let selected_bit_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::REVERSED);
    let bit_visualization_text = bit_visualization_lines(
        model.integer_width,
        display_value,
        bit_manipulation_mode,
        selected_bit_index,
        selected_bit_style,
    );

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

    let help_text = footer_help_text(model);
    frame.render_widget(Paragraph::new(help_text).centered(), help_area);

    if model.show_quit_confirmation {
        render_quit_confirmation(frame);
    }
}

fn footer_help_text(model: &BitTwiddleModel) -> Line<'static> {
    let mut controls = vec![
        ("Tab", "Mode"),
        ("m/M", "Display"),
        ("w/W", "Width"),
        ("Ctrl+C", "Clear"),
        ("q", "Quit"),
    ];

    match model.input_mode {
        InputMode::BitManipulation => {
            controls.extend([("Arrows", "Select bit"), ("Space", "Toggle")]);
        }
        InputMode::ExpressionInput => {
            controls.extend([
                ("Enter", "Submit"),
                ("Left/Right", "Move"),
                ("Up/Down", "History"),
            ]);
        }
        InputMode::HistorySelect => {
            controls.extend([("Up/Down", "Select"), ("Enter", "Copy")]);
        }
    }

    if model.error.is_some() {
        controls.push(("Esc", "Dismiss"));
    }

    let key_style = Style::default().fg(Color::Yellow).bold();
    let label_style = Style::default().dim();
    let separator_style = Style::default().dim();

    let mut spans = Vec::new();
    for (index, (key, label)) in controls.into_iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled("  |  ", separator_style));
        }

        spans.push(Span::styled(key, key_style));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(label, label_style));
    }

    Line::from(spans)
}

fn width_span<'a>(
    current_width: IntegerWidth,
    target_width: IntegerWidth,
    label: &'a str,
) -> Span<'a> {
    Span::styled(
        label,
        if current_width == target_width {
            Style::default().underlined()
        } else {
            Style::default().dim()
        },
    )
}

fn bit_visualization_lines(
    width: IntegerWidth,
    display_value: u64,
    bit_manipulation_mode: bool,
    selected_bit_index: usize,
    selected_bit_style: Style,
) -> Vec<Line<'static>> {
    (0..bit_line_count(width))
        .rev()
        .map(|line_num| {
            Line::from(
                bit_words_for_line(width, line_num)
                    .rev()
                    .flat_map(|(word_start, word_end)| {
                        let mut spans = vec![
                            Span::styled((word_end - 1).to_string(), Style::default().bold()),
                            Span::raw("  "),
                        ];

                        for bit_index in (word_start..word_end).rev() {
                            let bit = if (display_value >> bit_index) & 1 == 1 {
                                "1"
                            } else {
                                "0"
                            };
                            let bit_style = if bit_manipulation_mode
                                && bit_index as usize == selected_bit_index
                            {
                                selected_bit_style
                            } else {
                                Style::default()
                            };
                            spans.push(Span::styled(bit, bit_style));

                            if bit_index > word_start {
                                spans.push(Span::raw(if bit_index % 4 == 0 { "  " } else { " " }));
                            }
                        }

                        spans.push(Span::raw("  "));
                        spans
                    })
                    .collect::<Vec<Span>>(),
            )
        })
        .collect()
}

fn bit_line_count(width: IntegerWidth) -> usize {
    width.bits().div_ceil(32) as usize
}

fn bit_words_for_line(
    width: IntegerWidth,
    line_num: usize,
) -> impl DoubleEndedIterator<Item = (u32, u32)> {
    let line_start = line_num as u32 * 32;
    let line_end = ((line_num as u32 + 1) * 32).min(width.bits());
    (line_start..line_end).step_by(16).map(move |word_start| {
        let word_end = (word_start + 16).min(line_end);
        (word_start, word_end)
    })
}

fn hit_test_bit(width: IntegerWidth, area: Rect, column: u16, row: u16) -> Option<usize> {
    let inner_width = area.width.saturating_sub(2);
    let x = column.checked_sub(area.x + 1)?;
    let y = row.checked_sub(area.y + 1)?;

    let line_count = bit_line_count(width);
    if y as usize >= line_count || x >= inner_width {
        return None;
    }

    let line_num = line_count - 1 - y as usize;
    let line_width = bit_line_width(width, line_num);
    let line_start = inner_width.saturating_sub(line_width) / 2;
    let line_column = x.checked_sub(line_start)?;
    if line_column >= line_width {
        return None;
    }

    bit_index_at_line_column(width, line_num, line_column)
}

fn hit_test_input(area: Rect, column: u16, row: u16) -> Option<usize> {
    let inner_width = area.width.saturating_sub(2);
    let x = column.checked_sub(area.x + 1)?;
    let y = row.checked_sub(area.y + 1)?;

    if y == 0 && x < inner_width {
        Some(x as usize)
    } else {
        None
    }
}

fn hit_test_history(
    area: Rect,
    history_len: usize,
    column: u16,
    row: u16,
) -> Option<Option<usize>> {
    let inner_width = area.width.saturating_sub(2);
    let inner_height = area.height.saturating_sub(2);
    let x = column.checked_sub(area.x + 1)?;
    let y = row.checked_sub(area.y + 1)?;

    if x >= inner_width || y >= inner_height {
        return None;
    }

    let index = y as usize;
    Some((index < history_len).then_some(index))
}

fn bit_line_width(width: IntegerWidth, line_num: usize) -> u16 {
    let mut line_width = 0;
    for (word_start, word_end) in bit_words_for_line(width, line_num) {
        line_width += (word_end - 1).to_string().len() as u16 + 2;
        for bit_index in (word_start..word_end).rev() {
            line_width += 1;
            if bit_index > word_start {
                line_width += if bit_index % 4 == 0 { 2 } else { 1 };
            }
        }
        line_width += 2;
    }
    line_width
}

fn bit_index_at_line_column(
    width: IntegerWidth,
    line_num: usize,
    target_column: u16,
) -> Option<usize> {
    let mut column = 0;
    for (word_start, word_end) in bit_words_for_line(width, line_num).rev() {
        column += (word_end - 1).to_string().len() as u16 + 2;

        for bit_index in (word_start..word_end).rev() {
            if column == target_column {
                return Some(bit_index as usize);
            }

            column += 1;
            if bit_index > word_start {
                column += if bit_index % 4 == 0 { 2 } else { 1 };
            }
        }

        column += 2;
    }

    None
}

fn render_quit_confirmation(frame: &mut Frame) {
    let area = centered_rect(49, 5, frame.area());
    let dialog = Paragraph::new(Text::from(vec![
        Line::from("Quit BitTwiddle?"),
        Line::from(""),
        Line::from("Press Y to quit or any other key to continue."),
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

fn handle_event(frame_area: Rect) -> Result<Option<Message>, io::Error> {
    match event::read()? {
        Event::Key(key) => Ok(Some(Message::KeyPressed(key))),
        Event::Mouse(mouse) => Ok(Some(Message::MousePressed(mouse, frame_area))),
        _ => Ok(None),
    }
}

fn main() -> Result<(), io::Error> {
    let mut terminal = ratatui::init();
    let result = (|| {
        execute!(io::stdout(), EnableMouseCapture)?;
        let mut model: BitTwiddleModel = BitTwiddleModel::default();

        while model.running_state != RunningState::Done {
            // Render the current view
            terminal.draw(|f| view(&model, f))?;
            let frame_area = terminal.size()?.into();

            // Handle events and map to a Message
            let mut current_msg = handle_event(frame_area)?;

            // Process updates as long as they return a non-None message
            while current_msg.is_some() {
                current_msg = update(&mut model, current_msg.unwrap());
            }
        }

        Ok(())
    })();

    let cleanup_result = execute!(io::stdout(), DisableMouseCapture);
    ratatui::restore();
    result.and(cleanup_result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expression::{Operator, ParsedExpression};
    use ratatui::crossterm::event::KeyModifiers;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ctrl_key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::CONTROL)
    }

    fn left_click(column: u16, row: u16) -> MouseEvent {
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column,
            row,
            modifiers: KeyModifiers::NONE,
        }
    }

    fn hit_test_bit(area: Rect, column: u16, row: u16) -> Option<usize> {
        super::hit_test_bit(IntegerWidth::SixtyFour, area, column, row)
    }

    fn history_entry(value: u64) -> model::CalculationHistoryEntry {
        model::CalculationHistoryEntry {
            expression: ParsedExpression::Binary {
                operand1: value,
                operator: Operator::Add,
                operand2: value,
            },
            result: value + value,
        }
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
    fn ctrl_c_clears_input() {
        let model = BitTwiddleModel::default();

        assert_eq!(
            map_key_to_message(&model, ctrl_key(KeyCode::Char('c'))),
            Some(Message::ClearInput)
        );
    }

    #[test]
    fn w_and_uppercase_w_cycle_integer_width() {
        let model = BitTwiddleModel::default();

        assert_eq!(
            map_key_to_message(&model, key(KeyCode::Char('w'))),
            Some(Message::NextIntegerWidth)
        );
        assert_eq!(
            map_key_to_message(&model, key(KeyCode::Char('W'))),
            Some(Message::PrevIntegerWidth)
        );
    }

    #[test]
    fn plain_c_is_expression_input_text() {
        let model = BitTwiddleModel::default();

        assert_eq!(
            map_key_to_message(&model, key(KeyCode::Char('c'))),
            Some(Message::InsertInputChar('c'))
        );
    }

    #[test]
    fn plain_c_does_not_clear_outside_expression_input() {
        let bit_model = BitTwiddleModel {
            input_mode: InputMode::BitManipulation,
            ..Default::default()
        };
        let history_model = BitTwiddleModel {
            input_mode: InputMode::HistorySelect,
            ..Default::default()
        };

        assert_eq!(
            map_key_to_message(&bit_model, key(KeyCode::Char('c'))),
            None
        );
        assert_eq!(
            map_key_to_message(&history_model, key(KeyCode::Char('c'))),
            None
        );
    }

    #[test]
    fn lowercase_hex_c_can_be_entered_character_by_character() {
        let model = BitTwiddleModel::default();

        for c in "0xc + 1".chars() {
            assert_eq!(
                map_key_to_message(&model, key(KeyCode::Char(c))),
                Some(Message::InsertInputChar(c))
            );
        }
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
        assert_eq!(
            map_key_to_message(&model, key(KeyCode::Char('q'))),
            Some(Message::CancelQuit)
        );
        assert_eq!(
            map_key_to_message(&model, key(KeyCode::Esc)),
            Some(Message::CancelQuit)
        );
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

    #[test]
    fn bit_hit_test_maps_visible_top_row_bits() {
        let area = Rect::new(0, 0, 82, 4);

        assert_eq!(hit_test_bit(area, 5, 1), Some(63));
        assert_eq!(hit_test_bit(area, 7, 1), Some(62));
        assert_eq!(hit_test_bit(area, 11, 1), Some(60));
        assert_eq!(hit_test_bit(area, 14, 1), Some(59));
        assert_eq!(hit_test_bit(area, 78, 1), Some(32));
    }

    #[test]
    fn bit_hit_test_maps_visible_bottom_row_bits() {
        let area = Rect::new(0, 0, 82, 4);

        assert_eq!(hit_test_bit(area, 5, 2), Some(31));
        assert_eq!(hit_test_bit(area, 7, 2), Some(30));
        assert_eq!(hit_test_bit(area, 11, 2), Some(28));
        assert_eq!(hit_test_bit(area, 14, 2), Some(27));
        assert_eq!(hit_test_bit(area, 78, 2), Some(0));
    }

    #[test]
    fn bit_hit_test_ignores_labels_separators_borders_and_outside_area() {
        let area = Rect::new(0, 0, 82, 4);

        assert_eq!(hit_test_bit(area, 0, 1), None);
        assert_eq!(hit_test_bit(area, 1, 1), None);
        assert_eq!(hit_test_bit(area, 2, 1), None);
        assert_eq!(hit_test_bit(area, 3, 1), None);
        assert_eq!(hit_test_bit(area, 6, 1), None);
        assert_eq!(hit_test_bit(area, 82, 1), None);
        assert_eq!(hit_test_bit(area, 5, 0), None);
        assert_eq!(hit_test_bit(area, 5, 3), None);
    }

    #[test]
    fn bit_hit_test_uses_active_integer_width() {
        let area = Rect::new(0, 0, 82, 4);

        assert_eq!(
            super::hit_test_bit(IntegerWidth::Eight, area, 33, 1),
            Some(7)
        );
        assert_eq!(
            super::hit_test_bit(IntegerWidth::Eight, area, 48, 1),
            Some(0)
        );
        assert_eq!(super::hit_test_bit(IntegerWidth::Eight, area, 5, 1), None);
        assert_eq!(super::hit_test_bit(IntegerWidth::Eight, area, 34, 2), None);
    }

    #[test]
    fn left_click_on_bit_maps_to_toggle_bit() {
        let model = BitTwiddleModel::default();
        let frame_area = Rect::new(0, 0, 82, 12);

        assert_eq!(
            map_mouse_to_message(&model, left_click(5, 4), frame_area),
            Some(Message::ToggleBit(63))
        );
    }

    #[test]
    fn left_click_on_input_maps_to_focus_input_with_cursor_position() {
        let model = BitTwiddleModel::default();
        let frame_area = Rect::new(0, 0, 82, 16);

        assert_eq!(
            map_mouse_to_message(&model, left_click(6, 8), frame_area),
            Some(Message::FocusExpressionInput(5))
        );
    }

    #[test]
    fn input_hit_test_ignores_borders_and_outside_area() {
        let area = Rect::new(0, 7, 82, 3);

        assert_eq!(hit_test_input(area, 1, 8), Some(0));
        assert_eq!(hit_test_input(area, 0, 8), None);
        assert_eq!(hit_test_input(area, 81, 8), None);
        assert_eq!(hit_test_input(area, 1, 7), None);
        assert_eq!(hit_test_input(area, 1, 9), None);
    }

    #[test]
    fn left_click_on_history_item_maps_to_visible_history_index() {
        let model = BitTwiddleModel {
            calculation_history: vec![history_entry(1), history_entry(2), history_entry(3)],
            ..Default::default()
        };
        let frame_area = Rect::new(0, 0, 82, 16);

        assert_eq!(
            map_mouse_to_message(&model, left_click(1, 11), frame_area),
            Some(Message::FocusCalculationHistory(Some(0)))
        );
        assert_eq!(
            map_mouse_to_message(&model, left_click(1, 12), frame_area),
            Some(Message::FocusCalculationHistory(Some(1)))
        );
    }

    #[test]
    fn left_click_on_history_body_below_items_focuses_history_without_clicked_item() {
        let model = BitTwiddleModel {
            calculation_history: vec![history_entry(1), history_entry(2)],
            ..Default::default()
        };
        let frame_area = Rect::new(0, 0, 82, 16);

        assert_eq!(
            map_mouse_to_message(&model, left_click(1, 13), frame_area),
            Some(Message::FocusCalculationHistory(None))
        );
    }

    #[test]
    fn history_hit_test_ignores_borders_and_outside_area() {
        let area = Rect::new(0, 10, 82, 5);

        assert_eq!(hit_test_history(area, 1, 1, 11), Some(Some(0)));
        assert_eq!(hit_test_history(area, 1, 1, 12), Some(None));
        assert_eq!(hit_test_history(area, 1, 0, 11), None);
        assert_eq!(hit_test_history(area, 1, 81, 11), None);
        assert_eq!(hit_test_history(area, 1, 1, 10), None);
        assert_eq!(hit_test_history(area, 1, 1, 14), None);
    }

    #[test]
    fn mouse_mapping_ignores_non_left_down_and_quit_confirmation() {
        let frame_area = Rect::new(0, 0, 82, 12);
        let model = BitTwiddleModel::default();
        let confirming_quit = BitTwiddleModel {
            show_quit_confirmation: true,
            ..Default::default()
        };

        assert_eq!(
            map_mouse_to_message(
                &model,
                MouseEvent {
                    kind: MouseEventKind::Up(MouseButton::Left),
                    column: 5,
                    row: 4,
                    modifiers: KeyModifiers::NONE,
                },
                frame_area,
            ),
            None
        );
        assert_eq!(
            map_mouse_to_message(&confirming_quit, left_click(5, 4), frame_area),
            None
        );
    }
}
