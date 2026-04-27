use ratatui::{
    Frame,
    crossterm::{
        event::{
            self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, MouseButton,
            MouseEvent, MouseEventKind,
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

    hit_test_bit(bit_area, mouse.column, mouse.row)
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
        ("c", "Clear"),
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

fn hit_test_bit(area: Rect, column: u16, row: u16) -> Option<usize> {
    const BIT_LINE_WIDTH: u16 = 80;

    let inner_width = area.width.saturating_sub(2);
    let x = column.checked_sub(area.x + 1)?;
    let y = row.checked_sub(area.y + 1)?;

    if y > 1 || x >= inner_width {
        return None;
    }

    let line_start = inner_width.saturating_sub(BIT_LINE_WIDTH) / 2;
    let line_column = x.checked_sub(line_start)?;
    if line_column >= BIT_LINE_WIDTH {
        return None;
    }

    bit_index_at_line_column(if y == 0 { 1 } else { 0 }, line_column)
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

fn bit_index_at_line_column(line_num: usize, target_column: u16) -> Option<usize> {
    const BITS_PER_LINE: usize = 32;
    const BITS_PER_WORD: usize = 16;
    const BITS_PER_NIBBLE: usize = 4;

    let mut column = 0;
    for word_num in (0..2).rev() {
        let label = ((line_num * BITS_PER_LINE) + (word_num * BITS_PER_WORD)) + (BITS_PER_WORD - 1);
        column += label.to_string().len() as u16 + 2;

        for nibble_num in (0..4).rev() {
            for nibble_index in (0..4).rev() {
                let bit_index = (line_num * BITS_PER_LINE)
                    + (word_num * BITS_PER_WORD)
                    + (nibble_num * BITS_PER_NIBBLE)
                    + nibble_index;
                if column == target_column {
                    return Some(bit_index);
                }

                column += 1;
                column += if nibble_index == 0 {
                    if nibble_num == 0 { 0 } else { 2 }
                } else {
                    1
                };
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
    use crate::expression::Operator;
    use ratatui::crossterm::event::KeyModifiers;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn left_click(column: u16, row: u16) -> MouseEvent {
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column,
            row,
            modifiers: KeyModifiers::NONE,
        }
    }

    fn history_entry(value: u64) -> model::CalculationHistoryEntry {
        model::CalculationHistoryEntry {
            operand1: value,
            operator: Operator::Add,
            operand2: value,
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
