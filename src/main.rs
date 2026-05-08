use ratatui::{
    crossterm::{
        event::{
            self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
            KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
        },
        execute,
    },
    layout::Rect,
};
use std::io;

mod expression;
mod model;
mod view;

use model::{BitTwiddleModel, InputMode, RunningState};
use view::{
    app_layout, expression_input_scroll, footer_help_line_count, hit_test_bit, hit_test_history,
    hit_test_input, view,
};

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
        KeyCode::Char(c) if is_expression_input_char(c) => Some(Message::InsertInputChar(c)),
        _ => None,
    }
}

fn is_expression_input_char(c: char) -> bool {
    c.is_ascii_hexdigit()
        || matches!(
            c,
            'x' | 'X'
                | 'o'
                | 'O'
                | '+'
                | '-'
                | '*'
                | '/'
                | '%'
                | '<'
                | '>'
                | '|'
                | '&'
                | '^'
                | '~'
                | '['
                | ']'
                | ':'
        )
        || c.is_ascii_whitespace()
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

    let layout = app_layout(
        model.integer_width,
        frame_area,
        footer_help_line_count(model, frame_area.width),
    )?;

    let input_scroll = expression_input_scroll(model, layout.input_area);

    hit_test_bit(
        model.integer_width,
        layout.bits_per_line,
        layout.bit_area,
        mouse.column,
        mouse.row,
    )
    .map(Message::ToggleBit)
    .or_else(|| {
        hit_test_input(layout.input_area, input_scroll, mouse.column, mouse.row)
            .map(Message::FocusExpressionInput)
    })
    .or_else(|| {
        let expression_history_area = layout.expression_history_area?;
        hit_test_history(
            expression_history_area,
            model.calculation_history.len(),
            mouse.column,
            mouse.row,
        )
        .map(Message::FocusCalculationHistory)
    })
}

fn handle_event(frame_area: Rect) -> Result<Option<Message>, io::Error> {
    match event::read()? {
        Event::Key(key) if key.kind == KeyEventKind::Press => Ok(Some(Message::KeyPressed(key))),
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
    use crate::expression::{IntegerWidth, Operator, ParsedExpression};
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
    fn expression_input_accepts_only_ascii_expression_characters() {
        let model = BitTwiddleModel::default();

        for c in "0123456789aAbBcCdDeEfFxXoO+-*/<>|&^~[]:\t\n\r ".chars() {
            assert_eq!(
                map_key_to_message(&model, key(KeyCode::Char(c))),
                Some(Message::InsertInputChar(c))
            );
        }
    }

    #[test]
    fn expression_input_ignores_unsupported_characters() {
        let model = BitTwiddleModel::default();

        for c in ['g', 'G', '_', '(', ')', 'é', '１'] {
            assert_eq!(map_key_to_message(&model, key(KeyCode::Char(c))), None);
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
    fn narrow_bit_hit_test_matches_wrapped_rows() {
        let frame_area = Rect::new(0, 0, 40, 16);
        let layout = app_layout(IntegerWidth::SixtyFour, frame_area, 1).unwrap();
        let top_row = layout.bit_area.y + 1;
        let leftmost_top_bit = layout.bit_area.x + 13;

        assert_eq!(
            super::hit_test_bit(
                IntegerWidth::SixtyFour,
                layout.bits_per_line,
                layout.bit_area,
                leftmost_top_bit,
                top_row,
            ),
            Some(63)
        );
        assert_eq!(
            map_mouse_to_message(
                &BitTwiddleModel::default(),
                left_click(leftmost_top_bit, top_row),
                frame_area,
            ),
            Some(Message::ToggleBit(63))
        );
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
        let frame_area = Rect::new(0, 0, 82, 17);

        assert_eq!(
            map_mouse_to_message(&model, left_click(6, 8), frame_area),
            Some(Message::FocusExpressionInput(5))
        );
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
        let frame_area = Rect::new(0, 0, 82, 17);

        assert_eq!(
            map_mouse_to_message(&model, left_click(1, 13), frame_area),
            Some(Message::FocusCalculationHistory(None))
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
}
