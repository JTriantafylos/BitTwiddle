use ratatui::{
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind},
    layout::Rect,
};

pub mod expression;
pub mod model;
pub mod view;

use model::{BitTwiddleModel, DisplayMode, InputMode};
use view::{
    app_layout, expression_input_scroll, footer_help_line_count, hit_test_bit,
    hit_test_display_mode, hit_test_history, hit_test_input, hit_test_value_area, hit_test_width,
};

use expression::IntegerWidth;

#[derive(Debug, PartialEq, Eq)]
pub enum Message {
    KeyPressed(KeyEvent),
    MousePressed(MouseEvent, Rect),
    ShowNotification(String),
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
    SelectDisplayMode(DisplayMode),
    PrevDisplayMode,
    NextDisplayMode,
    SelectIntegerWidth(IntegerWidth),
    PrevIntegerWidth,
    NextIntegerWidth,
    DismissError,
    RequestQuit,
    CancelQuit,
    ConfirmQuit,
    CopyCurrentValue,
    PasteFromClipboard,
}

pub fn map_key_to_message(model: &BitTwiddleModel, key: KeyEvent) -> Option<Message> {
    if model.show_quit_confirmation() {
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
        _ => map_mode_key_to_message(model.input_mode(), key),
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
        KeyCode::Char(c) if is_expression_input_char(c) => Some(Message::InsertInputChar(c)),
        _ => None,
    }
}

pub(crate) fn is_expression_input_char(c: char) -> bool {
    if c.is_ascii_hexdigit() || c.is_ascii_whitespace() {
        return true;
    }

    matches!(
        c,
        // Prefixes: 0x, 0o, 0b
        'x' | 'X' | 'o' | 'O' | 'b' | 'B'
    ) || matches!(
        c,
        // Arithmetic operators
        '+' | '-' | '*' | '/' | '%'
    ) || matches!(
        c,
        // Bitwise operators
        '|' | '&' | '^' | '~'
    ) || matches!(
        c,
        // Shift operators
        '<' | '>'
    ) || matches!(
        c,
        // Bit range syntax
        '[' | ']' | ':'
    )
}

pub fn update(model: &mut BitTwiddleModel, msg: Message) -> Option<Message> {
    // Clear any existing notifications when handling a new message.
    let should_clear_notif = match msg {
        // Don't clear notifications on mouse-up, otherwise notifications created by mouse-down
        // events will be cleared immediately.
        Message::MousePressed(mouse, _) => matches!(mouse.kind, MouseEventKind::Down(_)),
        _ => true,
    };

    if should_clear_notif {
        model.clear_notification();
    }

    match msg {
        Message::KeyPressed(key) => map_key_to_message(model, key),
        Message::MousePressed(mouse, area) => map_mouse_to_message(model, mouse, area),
        Message::ShowNotification(text) => model.show_notification(text),
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
        Message::SelectDisplayMode(display_mode) => model.select_display_mode(display_mode),
        Message::PrevDisplayMode => model.prev_display_mode(),
        Message::NextDisplayMode => model.next_display_mode(),
        Message::SelectIntegerWidth(integer_width) => model.select_integer_width(integer_width),
        Message::PrevIntegerWidth => model.prev_integer_width(),
        Message::NextIntegerWidth => model.next_integer_width(),
        Message::DismissError => model.dismiss_error(),
        Message::RequestQuit => model.request_quit(),
        Message::CancelQuit => model.cancel_quit(),
        Message::ConfirmQuit => model.confirm_quit(),
        Message::CopyCurrentValue => model.copy_current_value(),
        Message::PasteFromClipboard => model.paste_from_clipboard(),
    }
}

pub fn map_mouse_to_message(
    model: &BitTwiddleModel,
    mouse: MouseEvent,
    frame_area: Rect,
) -> Option<Message> {
    if model.show_quit_confirmation() {
        return None;
    }
    let down = match mouse.kind {
        MouseEventKind::Down(button) => button,
        _ => return None,
    };
    if down == MouseButton::Middle {
        return Some(Message::PasteFromClipboard);
    }

    let layout = app_layout(
        model.integer_width(),
        frame_area,
        footer_help_line_count(model, frame_area.width),
    )?;

    let input_scroll = expression_input_scroll(model, layout.input_area);

    layout
        .mode_area
        .and_then(|area| hit_test_display_mode(area, mouse.column))
        .map(Message::SelectDisplayMode)
        .or_else(|| {
            layout
                .width_area
                .and_then(|area| hit_test_width(area, mouse.column))
                .map(Message::SelectIntegerWidth)
        })
        .or_else(|| {
            hit_test_value_area(layout.value_areas, mouse.column, mouse.row)
                .then_some(Message::CopyCurrentValue)
        })
        .or_else(|| {
            hit_test_bit(
                model.integer_width(),
                layout.bits_per_line,
                layout.bit_area,
                mouse.column,
                mouse.row,
            )
            .map(Message::ToggleBit)
        })
        .or_else(|| {
            hit_test_input(layout.input_area, input_scroll, mouse.column, mouse.row)
                .map(Message::FocusExpressionInput)
        })
        .or_else(|| {
            let expression_history_area = layout.expression_history_area?;
            hit_test_history(
                expression_history_area,
                model.calculation_history_len(),
                mouse.column,
                mouse.row,
            )
            .map(Message::FocusCalculationHistory)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expression::{Operator, ParsedExpression};

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
    fn global_keys_map_to_app_messages() {
        let model = BitTwiddleModel::default();

        assert_eq!(
            map_key_to_message(&model, key(KeyCode::Char('q'))),
            Some(Message::RequestQuit)
        );
        assert_eq!(
            map_key_to_message(&model, ctrl_key(KeyCode::Char('c'))),
            Some(Message::ClearInput)
        );
        assert_eq!(
            map_key_to_message(&model, key(KeyCode::Char('w'))),
            Some(Message::NextIntegerWidth)
        );
        assert_eq!(
            map_key_to_message(&model, key(KeyCode::Char('W'))),
            Some(Message::PrevIntegerWidth)
        );
    }

    fn middle_click() -> MouseEvent {
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Middle),
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        }
    }

    fn right_click() -> MouseEvent {
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Right),
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        }
    }

    fn mouse_up() -> MouseEvent {
        MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        }
    }

    #[test]
    fn middle_click_pastes_into_expression_input() {
        let model = BitTwiddleModel::default();
        let frame_area = Rect::new(0, 0, 82, 17);

        assert_eq!(
            map_mouse_to_message(&model, middle_click(), frame_area),
            Some(Message::PasteFromClipboard),
        );
    }

    #[test]
    fn middle_click_ignores_non_middle_buttons_and_non_down_events() {
        let model = BitTwiddleModel::default();
        let frame_area = Rect::new(0, 0, 82, 17);

        assert_eq!(
            map_mouse_to_message(&model, right_click(), frame_area),
            None,
        );
        assert_eq!(map_mouse_to_message(&model, mouse_up(), frame_area), None,);
    }

    #[test]
    fn middle_click_ignores_quit_confirmation() {
        let model = BitTwiddleModel {
            show_quit_confirmation: true,
            ..Default::default()
        };
        let frame_area = Rect::new(0, 0, 82, 17);

        assert_eq!(
            map_mouse_to_message(&model, middle_click(), frame_area),
            None,
        );
    }

    #[test]
    fn expression_input_accepts_only_ascii_expression_characters() {
        let model = BitTwiddleModel::default();

        for c in "0123456789aAbBcCdDeEfFxXoO+-*/<>|&^~[]:\t\n\r ".chars() {
            assert_eq!(
                map_key_to_message(&model, key(KeyCode::Char(c))),
                Some(Message::InsertInputChar(c))
            );
        }

        for c in ['g', 'G', '_', '(', ')', 'é', '１'] {
            assert_eq!(map_key_to_message(&model, key(KeyCode::Char(c))), None);
        }
    }

    #[test]
    fn mode_specific_keys_respect_active_input_mode() {
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
            map_key_to_message(&history_model, key(KeyCode::Up)),
            Some(Message::PrevCalculationHistory)
        );
        assert_eq!(
            map_key_to_message(&history_model, key(KeyCode::Down)),
            Some(Message::NextCalculationHistory)
        );
        assert_eq!(
            map_key_to_message(&history_model, key(KeyCode::Enter)),
            Some(Message::CopySelectedCalculationHistory)
        );
    }

    #[test]
    fn quit_confirmation_accepts_y_or_any_cancel_key() {
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
            map_key_to_message(&model, key(KeyCode::Char('q'))),
            Some(Message::CancelQuit)
        );
        assert_eq!(
            map_key_to_message(&model, key(KeyCode::Esc)),
            Some(Message::CancelQuit)
        );
    }

    #[test]
    fn mouse_mapping_targets_bits_input_and_history() {
        let frame_area = Rect::new(0, 0, 82, 17);
        let model = BitTwiddleModel {
            calculation_history: vec![history_entry(1), history_entry(2), history_entry(3)],
            ..Default::default()
        };

        assert_eq!(
            map_mouse_to_message(&model, left_click(5, 4), frame_area),
            Some(Message::ToggleBit(63))
        );
        assert_eq!(
            map_mouse_to_message(&model, left_click(6, 8), frame_area),
            Some(Message::FocusExpressionInput(5))
        );
        assert_eq!(
            map_mouse_to_message(&model, left_click(1, 11), frame_area),
            Some(Message::FocusCalculationHistory(Some(0)))
        );
    }

    #[test]
    fn mouse_mapping_targets_display_mode_area() {
        let frame_area = Rect::new(0, 0, 82, 17);
        let model = BitTwiddleModel::default();

        assert_eq!(
            map_mouse_to_message(&model, left_click(65, 1), frame_area),
            Some(Message::SelectDisplayMode(DisplayMode::Decimal))
        );
        assert_eq!(
            map_mouse_to_message(&model, left_click(66, 1), frame_area),
            Some(Message::SelectDisplayMode(DisplayMode::Signed))
        );
        assert_eq!(
            map_mouse_to_message(&model, left_click(67, 1), frame_area),
            Some(Message::SelectDisplayMode(DisplayMode::Hex))
        );
        assert_eq!(
            map_mouse_to_message(&model, left_click(68, 1), frame_area),
            Some(Message::SelectDisplayMode(DisplayMode::Octal))
        );
    }

    #[test]
    fn mouse_mapping_targets_integer_width_area() {
        let frame_area = Rect::new(0, 0, 82, 17);
        let model = BitTwiddleModel::default();

        assert_eq!(
            map_mouse_to_message(&model, left_click(71, 1), frame_area),
            Some(Message::SelectIntegerWidth(IntegerWidth::Eight))
        );
        assert_eq!(
            map_mouse_to_message(&model, left_click(73, 1), frame_area),
            Some(Message::SelectIntegerWidth(IntegerWidth::Sixteen))
        );
        assert_eq!(
            map_mouse_to_message(&model, left_click(76, 1), frame_area),
            Some(Message::SelectIntegerWidth(IntegerWidth::ThirtyTwo))
        );
        assert_eq!(
            map_mouse_to_message(&model, left_click(79, 1), frame_area),
            Some(Message::SelectIntegerWidth(IntegerWidth::SixtyFour))
        );
    }

    #[test]
    fn mouse_mapping_targets_value_area() {
        let frame_area = Rect::new(0, 0, 82, 17);
        let model = BitTwiddleModel::default();

        // Value area inner content is at row 1 (middle row of the 3-row value zone)
        assert_eq!(
            map_mouse_to_message(&model, left_click(2, 1), frame_area),
            Some(Message::CopyCurrentValue)
        );
        assert_eq!(
            map_mouse_to_message(&model, left_click(40, 1), frame_area),
            Some(Message::CopyCurrentValue)
        );
    }

    #[test]
    fn mouse_mapping_ignores_display_mode_and_width_in_narrow_terminals() {
        let frame_area = Rect::new(0, 0, 30, 12);
        let model = BitTwiddleModel::default();

        assert_eq!(
            map_mouse_to_message(&model, left_click(1, 1), frame_area),
            None
        );
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

    #[test]
    fn notification_clears_on_key_press() {
        let mut model = BitTwiddleModel {
            notification: Some("test".into()),
            ..Default::default()
        };

        update(&mut model, Message::KeyPressed(key(KeyCode::Char('a'))));

        assert_eq!(model.notification, None);
    }

    #[test]
    fn notification_clears_on_submit_expression() {
        let mut model = BitTwiddleModel {
            notification: Some("test".into()),
            expression_input_text: "1 + 1".into(),
            expression_input_cursor_index: 5,
            ..Default::default()
        };

        update(&mut model, Message::SubmitExpression);

        assert_eq!(model.notification, None);
    }

    #[test]
    fn notification_clears_on_bit_toggle() {
        let mut model = BitTwiddleModel {
            notification: Some("test".into()),
            ..Default::default()
        };

        update(&mut model, Message::ToggleSelectedBit);

        assert_eq!(model.notification, None);
    }

    #[test]
    fn notification_clears_on_display_mode_change() {
        let mut model = BitTwiddleModel {
            notification: Some("test".into()),
            ..Default::default()
        };

        update(&mut model, Message::NextDisplayMode);

        assert_eq!(model.notification, None);
    }

    #[test]
    fn notification_does_not_clear_on_mouse_drag() {
        let mut model = BitTwiddleModel {
            notification: Some("test".into()),
            ..Default::default()
        };

        let drag_event = MouseEvent {
            kind: MouseEventKind::Drag(MouseButton::Left),
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        };

        update(
            &mut model,
            Message::MousePressed(drag_event, Rect::new(0, 0, 82, 17)),
        );

        assert_eq!(model.notification, Some("test".into()));
    }

    #[test]
    fn notification_does_not_clear_on_mouse_move() {
        let mut model = BitTwiddleModel {
            notification: Some("test".into()),
            ..Default::default()
        };

        let move_event = MouseEvent {
            kind: MouseEventKind::Moved,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        };

        update(
            &mut model,
            Message::MousePressed(move_event, Rect::new(0, 0, 82, 17)),
        );

        assert_eq!(model.notification, Some("test".into()));
    }

    #[test]
    fn notification_does_not_clear_on_mouse_up() {
        let mut model = BitTwiddleModel {
            notification: Some("test".into()),
            ..Default::default()
        };

        let up_event = MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        };

        update(
            &mut model,
            Message::MousePressed(up_event, Rect::new(0, 0, 82, 17)),
        );

        assert_eq!(model.notification, Some("test".into()));
    }
}
