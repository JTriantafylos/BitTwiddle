use crate::{
    Message,
    expression::{Operator, compute, parse_expr},
};

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) enum RunningState {
    #[default]
    Running,
    Done,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) enum DisplayMode {
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

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) enum InputMode {
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

pub(crate) struct BitTwiddleModel {
    pub(crate) running_state: RunningState,
    pub(crate) value: u64,
    pub(crate) error: Option<String>,
    pub(crate) display_mode: DisplayMode,
    pub(crate) input_mode: InputMode,
    pub(crate) expression_input_text: String,
    pub(crate) expression_input_cursor_index: usize,
    pub(crate) expression_input_history: Vec<String>,
    pub(crate) expression_input_history_index: Option<usize>,
    pub(crate) calculation_history: Vec<(u64, Operator, u64, u64)>,
    pub(crate) selected_bit_index: usize,
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
    pub(crate) fn clear_input(&mut self) -> Option<Message> {
        self.value = 0;
        self.error = None;
        None
    }

    pub(crate) fn submit_expression(&mut self) -> Option<Message> {
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

    pub(crate) fn shift_input_cursor(&mut self, offset: isize) -> Option<Message> {
        self.expression_input_cursor_index = self
            .expression_input_cursor_index
            .saturating_add_signed(offset)
            .clamp(0, self.expression_input_text.len());
        None
    }

    pub(crate) fn insert_input_char(&mut self, c: char) -> Option<Message> {
        self.expression_input_text
            .insert(self.expression_input_cursor_index, c);
        Some(Message::ShiftInputCursor(1))
    }

    pub(crate) fn backspace_input_char(&mut self) -> Option<Message> {
        if !self.expression_input_text.is_empty() && self.expression_input_cursor_index > 0 {
            self.expression_input_text
                .remove(self.expression_input_cursor_index - 1);
            Some(Message::ShiftInputCursor(-1))
        } else {
            None
        }
    }

    pub(crate) fn delete_input_char(&mut self) -> Option<Message> {
        if !self.expression_input_text.is_empty()
            && self.expression_input_cursor_index < self.expression_input_text.len()
        {
            self.expression_input_text
                .remove(self.expression_input_cursor_index);
        }
        None
    }

    pub(crate) fn shift_selected_bit(&mut self, offset: isize) -> Option<Message> {
        self.selected_bit_index = self
            .selected_bit_index
            .saturating_add_signed(offset)
            .clamp(0, 63);
        None
    }

    pub(crate) fn toggle_selected_bit(&mut self) -> Option<Message> {
        self.value ^= 1_u64 << self.selected_bit_index;
        self.error = None;
        None
    }

    pub(crate) fn prev_input_mode(&mut self) -> Option<Message> {
        self.input_mode = self.input_mode.prev();
        None
    }

    pub(crate) fn next_input_mode(&mut self) -> Option<Message> {
        self.input_mode = self.input_mode.next();
        None
    }

    pub(crate) fn prev_display_mode(&mut self) -> Option<Message> {
        self.display_mode = self.display_mode.prev();
        None
    }

    pub(crate) fn next_display_mode(&mut self) -> Option<Message> {
        self.display_mode = self.display_mode.next();
        None
    }

    pub(crate) fn dismiss_error(&mut self) -> Option<Message> {
        self.error = None;
        None
    }

    pub(crate) fn quit(&mut self) -> Option<Message> {
        self.running_state = RunningState::Done;
        None
    }

    pub(crate) fn prev_input_history(&mut self) -> Option<Message> {
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

    pub(crate) fn next_input_history(&mut self) -> Option<Message> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn model_with_input(input: &str, cursor: usize) -> BitTwiddleModel {
        BitTwiddleModel {
            expression_input_text: input.to_string(),
            expression_input_cursor_index: cursor,
            ..Default::default()
        }
    }

    #[test]
    fn default_model_starts_in_expression_input_mode() {
        let model = BitTwiddleModel::default();

        assert_eq!(model.running_state, RunningState::Running);
        assert_eq!(model.value, 0);
        assert_eq!(model.error, None);
        assert_eq!(model.display_mode, DisplayMode::Decimal);
        assert_eq!(model.input_mode, InputMode::ExpressionInput);
        assert_eq!(model.expression_input_text, "");
        assert_eq!(model.expression_input_cursor_index, 0);
        assert_eq!(model.expression_input_history, Vec::<String>::new());
        assert_eq!(model.expression_input_history_index, None);
        assert_eq!(model.calculation_history, Vec::new());
        assert_eq!(model.selected_bit_index, 0);
    }

    #[test]
    fn display_mode_prev_and_next_cycle_through_modes() {
        assert_eq!(DisplayMode::Decimal.prev(), DisplayMode::Octal);
        assert_eq!(DisplayMode::Hex.prev(), DisplayMode::Decimal);
        assert_eq!(DisplayMode::Octal.prev(), DisplayMode::Hex);

        assert_eq!(DisplayMode::Decimal.next(), DisplayMode::Hex);
        assert_eq!(DisplayMode::Hex.next(), DisplayMode::Octal);
        assert_eq!(DisplayMode::Octal.next(), DisplayMode::Decimal);
    }

    #[test]
    fn input_mode_prev_and_next_cycle_through_modes() {
        assert_eq!(InputMode::BitManipulation.prev(), InputMode::HistorySelect);
        assert_eq!(
            InputMode::ExpressionInput.prev(),
            InputMode::BitManipulation
        );
        assert_eq!(InputMode::HistorySelect.prev(), InputMode::ExpressionInput);

        assert_eq!(
            InputMode::BitManipulation.next(),
            InputMode::ExpressionInput
        );
        assert_eq!(InputMode::ExpressionInput.next(), InputMode::HistorySelect);
        assert_eq!(InputMode::HistorySelect.next(), InputMode::BitManipulation);
    }

    #[test]
    fn clear_input_resets_value_and_error() {
        let mut model = BitTwiddleModel {
            value: 42,
            error: Some("problem".to_string()),
            ..Default::default()
        };

        assert_eq!(model.clear_input(), None);

        assert_eq!(model.value, 0);
        assert_eq!(model.error, None);
    }

    #[test]
    fn submit_expression_updates_value_and_histories_on_success() {
        let mut model = model_with_input("2 + 3", 5);

        assert_eq!(model.submit_expression(), None);

        assert_eq!(model.value, 5);
        assert_eq!(model.error, None);
        assert_eq!(model.expression_input_text, "");
        assert_eq!(model.expression_input_cursor_index, 0);
        assert_eq!(model.expression_input_history_index, None);
        assert_eq!(model.expression_input_history, vec!["2 + 3"]);
        assert_eq!(model.calculation_history, vec![(2, Operator::Add, 3, 5)]);
    }

    #[test]
    fn submit_expression_uses_current_value_for_omitted_left_operand() {
        let mut model = BitTwiddleModel {
            value: 10,
            expression_input_text: "+ 7".to_string(),
            expression_input_cursor_index: 3,
            ..Default::default()
        };

        model.submit_expression();

        assert_eq!(model.value, 17);
        assert_eq!(model.calculation_history, vec![(10, Operator::Add, 7, 17)]);
    }

    #[test]
    fn submit_expression_sets_parse_error_and_resets_input() {
        let mut model = model_with_input("nope", 4);

        assert_eq!(model.submit_expression(), None);

        assert_eq!(model.value, 0);
        assert!(model.error.as_deref().unwrap().starts_with("Parse error:"));
        assert_eq!(model.expression_input_text, "");
        assert_eq!(model.expression_input_cursor_index, 0);
        assert!(model.expression_input_history.is_empty());
        assert!(model.calculation_history.is_empty());
    }

    #[test]
    fn submit_expression_sets_computation_error_and_resets_input() {
        let mut model = model_with_input("5 / 0", 5);

        assert_eq!(model.submit_expression(), None);

        assert_eq!(model.value, 0);
        assert_eq!(
            model.error.as_deref(),
            Some("Computation error: Division by zero")
        );
        assert_eq!(model.expression_input_text, "");
        assert!(model.expression_input_history.is_empty());
        assert!(model.calculation_history.is_empty());
    }

    #[test]
    fn reset_expression_input_clears_text_cursor_and_history_selection() {
        let mut model = BitTwiddleModel {
            expression_input_text: "1 + 2".to_string(),
            expression_input_cursor_index: 5,
            expression_input_history_index: Some(1),
            ..Default::default()
        };

        model.reset_expression_input();

        assert_eq!(model.expression_input_text, "");
        assert_eq!(model.expression_input_cursor_index, 0);
        assert_eq!(model.expression_input_history_index, None);
    }

    #[test]
    fn shift_input_cursor_moves_within_input_bounds() {
        let mut model = model_with_input("12345", 2);

        assert_eq!(model.shift_input_cursor(2), None);
        assert_eq!(model.expression_input_cursor_index, 4);

        model.shift_input_cursor(isize::MAX);
        assert_eq!(model.expression_input_cursor_index, 5);

        model.shift_input_cursor(isize::MIN);
        assert_eq!(model.expression_input_cursor_index, 0);
    }

    #[test]
    fn insert_input_char_inserts_at_cursor_and_requests_cursor_shift() {
        let mut model = model_with_input("13", 1);

        assert_eq!(
            model.insert_input_char('2'),
            Some(Message::ShiftInputCursor(1))
        );

        assert_eq!(model.expression_input_text, "123");
        assert_eq!(model.expression_input_cursor_index, 1);
    }

    #[test]
    fn backspace_input_char_removes_character_before_cursor() {
        let mut model = model_with_input("123", 2);

        assert_eq!(
            model.backspace_input_char(),
            Some(Message::ShiftInputCursor(-1))
        );

        assert_eq!(model.expression_input_text, "13");
        assert_eq!(model.expression_input_cursor_index, 2);
    }

    #[test]
    fn backspace_input_char_does_nothing_at_start_or_empty_input() {
        let mut at_start = model_with_input("123", 0);
        let mut empty = model_with_input("", 0);

        assert_eq!(at_start.backspace_input_char(), None);
        assert_eq!(empty.backspace_input_char(), None);
        assert_eq!(at_start.expression_input_text, "123");
        assert_eq!(empty.expression_input_text, "");
    }

    #[test]
    fn delete_input_char_removes_character_at_cursor() {
        let mut model = model_with_input("123", 1);

        assert_eq!(model.delete_input_char(), None);

        assert_eq!(model.expression_input_text, "13");
        assert_eq!(model.expression_input_cursor_index, 1);
    }

    #[test]
    fn delete_input_char_does_nothing_at_end_or_empty_input() {
        let mut at_end = model_with_input("123", 3);
        let mut empty = model_with_input("", 0);

        assert_eq!(at_end.delete_input_char(), None);
        assert_eq!(empty.delete_input_char(), None);
        assert_eq!(at_end.expression_input_text, "123");
        assert_eq!(empty.expression_input_text, "");
    }

    #[test]
    fn shift_selected_bit_moves_within_bit_bounds() {
        let mut model = BitTwiddleModel {
            selected_bit_index: 30,
            ..Default::default()
        };

        assert_eq!(model.shift_selected_bit(32), None);
        assert_eq!(model.selected_bit_index, 62);

        model.shift_selected_bit(32);
        assert_eq!(model.selected_bit_index, 63);

        model.shift_selected_bit(isize::MIN);
        assert_eq!(model.selected_bit_index, 0);
    }

    #[test]
    fn toggle_selected_bit_flips_bit_and_clears_error() {
        let mut model = BitTwiddleModel {
            selected_bit_index: 3,
            error: Some("problem".to_string()),
            ..Default::default()
        };

        assert_eq!(model.toggle_selected_bit(), None);
        assert_eq!(model.value, 8);
        assert_eq!(model.error, None);

        model.toggle_selected_bit();
        assert_eq!(model.value, 0);
    }

    #[test]
    fn input_mode_methods_update_current_mode() {
        let mut model = BitTwiddleModel::default();

        assert_eq!(model.prev_input_mode(), None);
        assert_eq!(model.input_mode, InputMode::BitManipulation);

        assert_eq!(model.next_input_mode(), None);
        assert_eq!(model.input_mode, InputMode::ExpressionInput);
    }

    #[test]
    fn display_mode_methods_update_current_mode() {
        let mut model = BitTwiddleModel::default();

        assert_eq!(model.prev_display_mode(), None);
        assert_eq!(model.display_mode, DisplayMode::Octal);

        assert_eq!(model.next_display_mode(), None);
        assert_eq!(model.display_mode, DisplayMode::Decimal);
    }

    #[test]
    fn dismiss_error_clears_error() {
        let mut model = BitTwiddleModel {
            error: Some("problem".to_string()),
            ..Default::default()
        };

        assert_eq!(model.dismiss_error(), None);

        assert_eq!(model.error, None);
    }

    #[test]
    fn quit_sets_running_state_to_done() {
        let mut model = BitTwiddleModel::default();

        assert_eq!(model.quit(), None);

        assert_eq!(model.running_state, RunningState::Done);
    }

    #[test]
    fn next_input_history_loads_most_recent_entries() {
        let mut model = BitTwiddleModel {
            expression_input_history: vec![
                "1 + 1".to_string(),
                "2 + 2".to_string(),
                "3 + 3".to_string(),
            ],
            ..Default::default()
        };

        assert_eq!(model.next_input_history(), None);
        assert_eq!(model.expression_input_text, "3 + 3");
        assert_eq!(model.expression_input_cursor_index, 5);
        assert_eq!(model.expression_input_history_index, Some(0));

        model.next_input_history();
        assert_eq!(model.expression_input_text, "2 + 2");
        assert_eq!(model.expression_input_history_index, Some(1));

        model.next_input_history();
        model.next_input_history();
        assert_eq!(model.expression_input_text, "1 + 1");
        assert_eq!(model.expression_input_history_index, Some(2));
    }

    #[test]
    fn next_input_history_does_nothing_when_history_is_empty() {
        let mut model = BitTwiddleModel::default();

        assert_eq!(model.next_input_history(), None);

        assert_eq!(model.expression_input_text, "");
        assert_eq!(model.expression_input_history_index, None);
    }

    #[test]
    fn prev_input_history_moves_toward_current_input_and_then_clears() {
        let mut model = BitTwiddleModel {
            expression_input_history: vec![
                "1 + 1".to_string(),
                "2 + 2".to_string(),
                "3 + 3".to_string(),
            ],
            expression_input_history_index: Some(2),
            ..Default::default()
        };
        model.load_input_history(2);

        assert_eq!(model.prev_input_history(), None);
        assert_eq!(model.expression_input_text, "2 + 2");
        assert_eq!(model.expression_input_history_index, Some(1));

        model.prev_input_history();
        assert_eq!(model.expression_input_text, "3 + 3");
        assert_eq!(model.expression_input_history_index, Some(0));

        model.prev_input_history();
        assert_eq!(model.expression_input_text, "");
        assert_eq!(model.expression_input_cursor_index, 0);
        assert_eq!(model.expression_input_history_index, None);
    }

    #[test]
    fn prev_input_history_does_nothing_without_selected_history() {
        let mut model = BitTwiddleModel {
            expression_input_text: "current".to_string(),
            expression_input_cursor_index: 7,
            expression_input_history: vec!["1 + 1".to_string()],
            ..Default::default()
        };

        assert_eq!(model.prev_input_history(), None);

        assert_eq!(model.expression_input_text, "current");
        assert_eq!(model.expression_input_cursor_index, 7);
        assert_eq!(model.expression_input_history_index, None);
    }

    #[test]
    fn load_input_history_loads_reversed_index_and_moves_cursor_to_end() {
        let mut model = BitTwiddleModel {
            expression_input_history: vec![
                "1 + 1".to_string(),
                "2 + 2".to_string(),
                "3 + 3".to_string(),
            ],
            ..Default::default()
        };

        model.load_input_history(1);

        assert_eq!(model.expression_input_text, "2 + 2");
        assert_eq!(model.expression_input_cursor_index, 5);
        assert_eq!(model.expression_input_history_index, Some(1));
    }
}
