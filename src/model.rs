use crate::{
    Message,
    expression::{IntegerWidth, ParsedExpression, compute_expr, parse_expr},
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CalculationHistoryEntry {
    pub(crate) expression: ParsedExpression,
    pub(crate) result: u64,
}

#[derive(Default)]
pub(crate) struct BitTwiddleModel {
    pub(crate) running_state: RunningState,
    pub(crate) show_quit_confirmation: bool,
    pub(crate) value: u64,
    pub(crate) error: Option<String>,
    pub(crate) integer_width: IntegerWidth,
    pub(crate) display_mode: DisplayMode,
    pub(crate) input_mode: InputMode,
    pub(crate) expression_input_text: String,
    pub(crate) expression_input_cursor_index: usize,
    pub(crate) expression_input_history: Vec<String>,
    pub(crate) expression_input_history_index: Option<usize>,
    pub(crate) calculation_history: Vec<CalculationHistoryEntry>,
    pub(crate) calculation_history_selection_index: Option<usize>,
    pub(crate) selected_bit_index: usize,
}

impl BitTwiddleModel {
    pub(crate) fn clear_input(&mut self) -> Option<Message> {
        self.value = 0;
        self.error = None;
        None
    }

    pub(crate) fn submit_expression(&mut self) -> Option<Message> {
        match parse_expr(
            self.integer_width,
            self.value & self.integer_width.mask(),
            &self.expression_input_text,
        ) {
            Ok(expression) => match compute_expr(self.integer_width, expression) {
                Ok(result) => {
                    self.value = result & self.integer_width.mask();
                    self.error = None;
                    self.expression_input_history
                        .push(self.expression_input_text.clone());
                    self.calculation_history
                        .push(CalculationHistoryEntry { expression, result });
                    self.reset_expression_input();
                }
                Err(e) => {
                    self.error = Some(format!("Computation error: {}", e));
                }
            },
            Err(e) => {
                self.error = Some(format!("Parse error: {}", e));
            }
        };
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
        let max_bit_index = self.max_bit_index();
        self.selected_bit_index = self
            .selected_bit_index
            .saturating_add_signed(offset)
            .clamp(0, max_bit_index);
        None
    }

    pub(crate) fn toggle_selected_bit(&mut self) -> Option<Message> {
        self.value ^= 1_u64 << self.selected_bit_index;
        self.value &= self.integer_width.mask();
        self.error = None;
        None
    }

    pub(crate) fn toggle_bit(&mut self, bit_index: usize) -> Option<Message> {
        self.selected_bit_index = bit_index.clamp(0, self.max_bit_index());
        self.input_mode = InputMode::BitManipulation;
        self.sync_calculation_history_selection();
        self.toggle_selected_bit()
    }

    pub(crate) fn focus_expression_input(&mut self, cursor_index: usize) -> Option<Message> {
        self.input_mode = InputMode::ExpressionInput;
        self.expression_input_cursor_index =
            cursor_index.clamp(0, self.expression_input_text.len());
        self.sync_calculation_history_selection();
        None
    }

    pub(crate) fn focus_calculation_history(
        &mut self,
        selection_index: Option<usize>,
    ) -> Option<Message> {
        self.input_mode = InputMode::HistorySelect;
        self.calculation_history_selection_index =
            selection_index.filter(|index| *index < self.calculation_history.len());

        if self.calculation_history_selection_index.is_none() {
            self.sync_calculation_history_selection();
        }

        None
    }

    pub(crate) fn prev_input_mode(&mut self) -> Option<Message> {
        self.input_mode = self.input_mode.prev();
        self.sync_calculation_history_selection();
        None
    }

    pub(crate) fn next_input_mode(&mut self) -> Option<Message> {
        self.input_mode = self.input_mode.next();
        self.sync_calculation_history_selection();
        None
    }

    fn sync_calculation_history_selection(&mut self) {
        if self.input_mode == InputMode::HistorySelect && !self.calculation_history.is_empty() {
            self.calculation_history_selection_index = Some(0);
        } else {
            self.calculation_history_selection_index = None;
        }
    }

    pub(crate) fn prev_display_mode(&mut self) -> Option<Message> {
        self.display_mode = self.display_mode.prev();
        None
    }

    pub(crate) fn next_display_mode(&mut self) -> Option<Message> {
        self.display_mode = self.display_mode.next();
        None
    }

    pub(crate) fn prev_integer_width(&mut self) -> Option<Message> {
        self.set_integer_width(self.integer_width.prev());
        None
    }

    pub(crate) fn next_integer_width(&mut self) -> Option<Message> {
        self.set_integer_width(self.integer_width.next());
        None
    }

    fn set_integer_width(&mut self, integer_width: IntegerWidth) {
        self.integer_width = integer_width;
        self.value &= self.integer_width.mask();
        self.selected_bit_index = self.selected_bit_index.clamp(0, self.max_bit_index());
        self.error = None;
    }

    pub(crate) fn max_bit_index(&self) -> usize {
        self.integer_width.bits() as usize - 1
    }

    pub(crate) fn dismiss_error(&mut self) -> Option<Message> {
        self.error = None;
        None
    }

    pub(crate) fn request_quit(&mut self) -> Option<Message> {
        self.show_quit_confirmation = true;
        None
    }

    pub(crate) fn cancel_quit(&mut self) -> Option<Message> {
        self.show_quit_confirmation = false;
        None
    }

    pub(crate) fn confirm_quit(&mut self) -> Option<Message> {
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

    pub(crate) fn prev_calculation_history(&mut self) -> Option<Message> {
        if self.calculation_history.is_empty() {
            self.calculation_history_selection_index = None;
        } else {
            self.calculation_history_selection_index = Some(
                self.calculation_history_selection_index
                    .unwrap_or(0)
                    .saturating_sub(1),
            );
        }
        None
    }

    pub(crate) fn next_calculation_history(&mut self) -> Option<Message> {
        if self.calculation_history.is_empty() {
            self.calculation_history_selection_index = None;
        } else {
            let last_index = self.calculation_history.len() - 1;
            self.calculation_history_selection_index = Some(
                self.calculation_history_selection_index
                    .unwrap_or(0)
                    .saturating_add(1)
                    .clamp(0, last_index),
            );
        }
        None
    }

    pub(crate) fn copy_selected_calculation_history(&mut self) -> Option<Message> {
        if let Some(entry) = self.selected_calculation_history_entry() {
            self.expression_input_text = self.format_calculation_history_expression(entry);
            self.expression_input_cursor_index = self.expression_input_text.len();
            self.expression_input_history_index = None;
            self.input_mode = InputMode::ExpressionInput;
            self.calculation_history_selection_index = None;
        }
        None
    }

    fn selected_calculation_history_entry(&self) -> Option<&CalculationHistoryEntry> {
        let index = self.calculation_history_selection_index?;
        let storage_index = self
            .calculation_history
            .len()
            .checked_sub(index.saturating_add(1))?;
        self.calculation_history.get(storage_index)
    }

    pub(crate) fn format_calculation_history_expression(
        &self,
        entry: &CalculationHistoryEntry,
    ) -> String {
        match entry.expression {
            ParsedExpression::Literal { value } => self.format_operand(value),
            ParsedExpression::Binary {
                operand1,
                operator,
                operand2,
            } => {
                format!(
                    "{} {} {}",
                    self.format_operand(operand1),
                    operator,
                    self.format_operand(operand2)
                )
            }
            ParsedExpression::UnaryNot { operand } => {
                format!("~{}", self.format_operand(operand))
            }
            ParsedExpression::BitRangeMask {
                operand,
                start_bit,
                end_bit,
            } => {
                if end_bit == start_bit + 1 {
                    format!("{}[{}]", self.format_operand(operand), start_bit)
                } else {
                    format!(
                        "{}[{}:{}]",
                        self.format_operand(operand),
                        start_bit,
                        end_bit
                    )
                }
            }
        }
    }

    fn format_operand(&self, operand: u64) -> String {
        match self.display_mode {
            DisplayMode::Decimal => operand.to_string(),
            DisplayMode::Hex => format!("{:#X}", operand),
            DisplayMode::Octal => format!("{:#o}", operand),
        }
    }

    pub(crate) fn format_calculation_history_entry(
        &self,
        entry: &CalculationHistoryEntry,
    ) -> String {
        match self.display_mode {
            DisplayMode::Decimal => format!(
                "{} = {}",
                self.format_calculation_history_expression(entry),
                entry.result
            ),
            DisplayMode::Hex => format!(
                "{} = {:#X}",
                self.format_calculation_history_expression(entry),
                entry.result
            ),
            DisplayMode::Octal => format!(
                "{} = {:#o}",
                self.format_calculation_history_expression(entry),
                entry.result
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expression::Operator;

    fn model_with_input(input: &str, cursor: usize) -> BitTwiddleModel {
        BitTwiddleModel {
            expression_input_text: input.to_string(),
            expression_input_cursor_index: cursor,
            ..Default::default()
        }
    }

    fn binary_history_entry(
        operand1: u64,
        operator: Operator,
        operand2: u64,
        result: u64,
    ) -> CalculationHistoryEntry {
        CalculationHistoryEntry {
            expression: ParsedExpression::Binary {
                operand1,
                operator,
                operand2,
            },
            result,
        }
    }

    #[test]
    fn default_model_starts_in_expression_input_mode() {
        let model = BitTwiddleModel::default();

        assert_eq!(model.running_state, RunningState::Running);
        assert!(!model.show_quit_confirmation);
        assert_eq!(model.value, 0);
        assert_eq!(model.error, None);
        assert_eq!(model.integer_width, IntegerWidth::SixtyFour);
        assert_eq!(model.display_mode, DisplayMode::Decimal);
        assert_eq!(model.input_mode, InputMode::ExpressionInput);
        assert_eq!(model.expression_input_text, "");
        assert_eq!(model.expression_input_cursor_index, 0);
        assert_eq!(model.expression_input_history, Vec::<String>::new());
        assert_eq!(model.expression_input_history_index, None);
        assert_eq!(model.calculation_history, Vec::new());
        assert_eq!(model.calculation_history_selection_index, None);
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
    fn integer_width_methods_update_value_error_and_selected_bit() {
        let mut model = BitTwiddleModel {
            value: 0x1ff,
            selected_bit_index: 63,
            error: Some("problem".to_string()),
            ..Default::default()
        };

        assert_eq!(model.next_integer_width(), None);

        assert_eq!(model.integer_width, IntegerWidth::Eight);
        assert_eq!(model.value, 0xff);
        assert_eq!(model.selected_bit_index, 7);
        assert_eq!(model.error, None);

        assert_eq!(model.prev_integer_width(), None);
        assert_eq!(model.integer_width, IntegerWidth::SixtyFour);
        assert_eq!(model.value, 0xff);
        assert_eq!(model.selected_bit_index, 7);
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
        assert_eq!(
            model.calculation_history,
            vec![binary_history_entry(2, Operator::Add, 3, 5)]
        );
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
        assert_eq!(
            model.calculation_history,
            vec![binary_history_entry(10, Operator::Add, 7, 17)]
        );
    }

    #[test]
    fn submit_expression_sets_value_from_single_number() {
        let mut model = BitTwiddleModel {
            value: 99,
            expression_input_text: "32".to_string(),
            expression_input_cursor_index: 2,
            ..Default::default()
        };

        model.submit_expression();

        assert_eq!(model.value, 32);
        assert_eq!(
            model.calculation_history,
            vec![CalculationHistoryEntry {
                expression: ParsedExpression::Literal { value: 32 },
                result: 32,
            }]
        );
    }

    #[test]
    fn submit_expression_supports_unary_not() {
        let mut model = model_with_input("~5", 2);

        model.submit_expression();

        assert_eq!(model.value, !5);
        assert_eq!(
            model.calculation_history,
            vec![CalculationHistoryEntry {
                expression: ParsedExpression::UnaryNot { operand: 5 },
                result: !5,
            }]
        );
    }

    #[test]
    fn submit_expression_uses_active_integer_width() {
        let mut model = BitTwiddleModel {
            integer_width: IntegerWidth::Eight,
            expression_input_text: "~5".to_string(),
            expression_input_cursor_index: 2,
            ..Default::default()
        };

        model.submit_expression();

        assert_eq!(model.value, 0xfa);
        assert_eq!(
            model.calculation_history,
            vec![CalculationHistoryEntry {
                expression: ParsedExpression::UnaryNot { operand: 5 },
                result: 0xfa,
            }]
        );
    }

    #[test]
    fn submit_expression_supports_bit_range_mask() {
        let mut model = model_with_input("33[0:2]", 7);

        model.submit_expression();

        assert_eq!(model.value, 1);
        assert_eq!(
            model.calculation_history,
            vec![CalculationHistoryEntry {
                expression: ParsedExpression::BitRangeMask {
                    operand: 33,
                    start_bit: 0,
                    end_bit: 2,
                },
                result: 1,
            }]
        );
    }

    #[test]
    fn submit_expression_supports_single_bit_mask() {
        let mut model = model_with_input("33[5]", 5);

        model.submit_expression();

        assert_eq!(model.value, 32);
        assert_eq!(
            model.calculation_history,
            vec![CalculationHistoryEntry {
                expression: ParsedExpression::BitRangeMask {
                    operand: 33,
                    start_bit: 5,
                    end_bit: 6,
                },
                result: 32,
            }]
        );
    }

    #[test]
    fn submit_expression_sets_parse_error_and_keeps_input_for_editing() {
        let mut model = model_with_input("nope", 4);

        assert_eq!(model.submit_expression(), None);

        assert_eq!(model.value, 0);
        assert!(model.error.as_deref().unwrap().starts_with("Parse error:"));
        assert_eq!(model.expression_input_text, "nope");
        assert_eq!(model.expression_input_cursor_index, 4);
        assert!(model.expression_input_history.is_empty());
        assert!(model.calculation_history.is_empty());
    }

    #[test]
    fn submit_expression_sets_computation_error_and_keeps_input_for_editing() {
        let mut model = model_with_input("5 / 0", 5);

        assert_eq!(model.submit_expression(), None);

        assert_eq!(model.value, 0);
        assert_eq!(
            model.error.as_deref(),
            Some("Computation error: Division by zero")
        );
        assert_eq!(model.expression_input_text, "5 / 0");
        assert_eq!(model.expression_input_cursor_index, 5);
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
    fn shift_selected_bit_clamps_to_active_integer_width() {
        let mut model = BitTwiddleModel {
            integer_width: IntegerWidth::Eight,
            selected_bit_index: 6,
            ..Default::default()
        };

        assert_eq!(model.shift_selected_bit(32), None);

        assert_eq!(model.selected_bit_index, 7);
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
    fn toggle_bit_selects_enters_bit_mode_toggles_and_clears_error() {
        let mut model = BitTwiddleModel {
            error: Some("problem".to_string()),
            input_mode: InputMode::HistorySelect,
            calculation_history: vec![binary_history_entry(1, Operator::Add, 2, 3)],
            calculation_history_selection_index: Some(0),
            ..Default::default()
        };

        assert_eq!(model.toggle_bit(5), None);

        assert_eq!(model.selected_bit_index, 5);
        assert_eq!(model.input_mode, InputMode::BitManipulation);
        assert_eq!(model.calculation_history_selection_index, None);
        assert_eq!(model.value, 1 << 5);
        assert_eq!(model.error, None);
    }

    #[test]
    fn toggle_bit_clamps_and_masks_to_active_integer_width() {
        let mut model = BitTwiddleModel {
            integer_width: IntegerWidth::Eight,
            value: 0xff,
            ..Default::default()
        };

        assert_eq!(model.toggle_bit(63), None);

        assert_eq!(model.selected_bit_index, 7);
        assert_eq!(model.value, 0x7f);
    }

    #[test]
    fn focus_expression_input_enters_input_mode_and_clamps_cursor() {
        let mut model = BitTwiddleModel {
            input_mode: InputMode::HistorySelect,
            expression_input_text: "12 + 34".to_string(),
            expression_input_cursor_index: 2,
            calculation_history: vec![binary_history_entry(1, Operator::Add, 2, 3)],
            calculation_history_selection_index: Some(0),
            ..Default::default()
        };

        assert_eq!(model.focus_expression_input(100), None);

        assert_eq!(model.input_mode, InputMode::ExpressionInput);
        assert_eq!(model.expression_input_text, "12 + 34");
        assert_eq!(model.expression_input_cursor_index, 7);
        assert_eq!(model.calculation_history_selection_index, None);
    }

    #[test]
    fn focus_calculation_history_selects_clicked_visible_index() {
        let mut model = BitTwiddleModel {
            input_mode: InputMode::ExpressionInput,
            calculation_history: vec![
                binary_history_entry(1, Operator::Add, 1, 2),
                binary_history_entry(2, Operator::Add, 2, 4),
            ],
            ..Default::default()
        };

        assert_eq!(model.focus_calculation_history(Some(1)), None);

        assert_eq!(model.input_mode, InputMode::HistorySelect);
        assert_eq!(model.calculation_history_selection_index, Some(1));
    }

    #[test]
    fn focus_calculation_history_without_clicked_item_uses_default_selection() {
        let mut model = BitTwiddleModel {
            calculation_history: vec![binary_history_entry(1, Operator::Add, 2, 3)],
            ..Default::default()
        };

        assert_eq!(model.focus_calculation_history(None), None);

        assert_eq!(model.input_mode, InputMode::HistorySelect);
        assert_eq!(model.calculation_history_selection_index, Some(0));
    }

    #[test]
    fn focus_calculation_history_without_history_leaves_selection_empty() {
        let mut model = BitTwiddleModel::default();

        assert_eq!(model.focus_calculation_history(Some(0)), None);

        assert_eq!(model.input_mode, InputMode::HistorySelect);
        assert_eq!(model.calculation_history_selection_index, None);
    }

    #[test]
    fn input_mode_methods_update_current_mode() {
        let mut model = BitTwiddleModel::default();

        assert_eq!(model.prev_input_mode(), None);
        assert_eq!(model.input_mode, InputMode::BitManipulation);
        assert_eq!(model.calculation_history_selection_index, None);

        assert_eq!(model.next_input_mode(), None);
        assert_eq!(model.input_mode, InputMode::ExpressionInput);
        assert_eq!(model.calculation_history_selection_index, None);
    }

    #[test]
    fn entering_history_select_initializes_selection_to_newest_entry() {
        let mut model = BitTwiddleModel {
            calculation_history: vec![binary_history_entry(1, Operator::Add, 2, 3)],
            ..Default::default()
        };

        assert_eq!(model.next_input_mode(), None);

        assert_eq!(model.input_mode, InputMode::HistorySelect);
        assert_eq!(model.calculation_history_selection_index, Some(0));

        model.next_input_mode();

        assert_eq!(model.input_mode, InputMode::BitManipulation);
        assert_eq!(model.calculation_history_selection_index, None);
    }

    #[test]
    fn entering_history_select_without_history_leaves_selection_empty() {
        let mut model = BitTwiddleModel::default();

        assert_eq!(model.next_input_mode(), None);

        assert_eq!(model.input_mode, InputMode::HistorySelect);
        assert_eq!(model.calculation_history_selection_index, None);
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
    fn request_quit_shows_confirmation() {
        let mut model = BitTwiddleModel::default();

        assert_eq!(model.request_quit(), None);

        assert!(model.show_quit_confirmation);
        assert_eq!(model.running_state, RunningState::Running);
    }

    #[test]
    fn cancel_quit_hides_confirmation() {
        let mut model = BitTwiddleModel {
            show_quit_confirmation: true,
            ..Default::default()
        };

        assert_eq!(model.cancel_quit(), None);

        assert!(!model.show_quit_confirmation);
        assert_eq!(model.running_state, RunningState::Running);
    }

    #[test]
    fn confirm_quit_sets_running_state_to_done() {
        let mut model = BitTwiddleModel::default();

        assert_eq!(model.confirm_quit(), None);

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

    #[test]
    fn calculation_history_selection_moves_newest_first_and_clamps() {
        let mut model = BitTwiddleModel {
            calculation_history: vec![
                binary_history_entry(1, Operator::Add, 1, 2),
                binary_history_entry(2, Operator::Add, 2, 4),
                binary_history_entry(3, Operator::Add, 3, 6),
            ],
            calculation_history_selection_index: Some(0),
            ..Default::default()
        };

        assert_eq!(model.prev_calculation_history(), None);
        assert_eq!(model.calculation_history_selection_index, Some(0));

        model.next_calculation_history();
        assert_eq!(model.calculation_history_selection_index, Some(1));

        model.next_calculation_history();
        model.next_calculation_history();
        assert_eq!(model.calculation_history_selection_index, Some(2));

        model.prev_calculation_history();
        assert_eq!(model.calculation_history_selection_index, Some(1));
    }

    #[test]
    fn calculation_history_selection_does_nothing_without_history() {
        let mut model = BitTwiddleModel::default();

        assert_eq!(model.next_calculation_history(), None);
        assert_eq!(model.calculation_history_selection_index, None);

        assert_eq!(model.prev_calculation_history(), None);
        assert_eq!(model.calculation_history_selection_index, None);

        assert_eq!(model.copy_selected_calculation_history(), None);
        assert_eq!(model.input_mode, InputMode::ExpressionInput);
        assert_eq!(model.expression_input_text, "");
    }

    #[test]
    fn copy_selected_calculation_history_uses_decimal_display_mode() {
        let mut model = BitTwiddleModel {
            input_mode: InputMode::HistorySelect,
            expression_input_text: "current".to_string(),
            expression_input_cursor_index: 7,
            expression_input_history_index: Some(0),
            calculation_history: vec![
                binary_history_entry(1, Operator::Add, 1, 2),
                binary_history_entry(10, Operator::ShiftLeft, 2, 40),
            ],
            calculation_history_selection_index: Some(0),
            ..Default::default()
        };

        assert_eq!(model.copy_selected_calculation_history(), None);

        assert_eq!(model.expression_input_text, "10 << 2");
        assert_eq!(model.expression_input_cursor_index, 7);
        assert_eq!(model.expression_input_history_index, None);
        assert_eq!(model.input_mode, InputMode::ExpressionInput);
        assert_eq!(model.calculation_history_selection_index, None);
    }

    #[test]
    fn copy_selected_calculation_history_uses_hex_display_mode() {
        let mut model = BitTwiddleModel {
            display_mode: DisplayMode::Hex,
            input_mode: InputMode::HistorySelect,
            calculation_history: vec![binary_history_entry(10, Operator::BitOr, 15, 15)],
            calculation_history_selection_index: Some(0),
            ..Default::default()
        };

        model.copy_selected_calculation_history();

        assert_eq!(model.expression_input_text, "0xA | 0xF");
    }

    #[test]
    fn copy_selected_calculation_history_uses_octal_display_mode() {
        let mut model = BitTwiddleModel {
            display_mode: DisplayMode::Octal,
            input_mode: InputMode::HistorySelect,
            calculation_history: vec![binary_history_entry(8, Operator::BitAnd, 7, 0)],
            calculation_history_selection_index: Some(0),
            ..Default::default()
        };

        model.copy_selected_calculation_history();

        assert_eq!(model.expression_input_text, "0o10 & 0o7");
    }

    #[test]
    fn copy_selected_calculation_history_formats_unary_and_range_expressions() {
        let mut model = BitTwiddleModel {
            display_mode: DisplayMode::Hex,
            input_mode: InputMode::HistorySelect,
            calculation_history: vec![
                CalculationHistoryEntry {
                    expression: ParsedExpression::UnaryNot { operand: 5 },
                    result: !5,
                },
                CalculationHistoryEntry {
                    expression: ParsedExpression::BitRangeMask {
                        operand: 33,
                        start_bit: 5,
                        end_bit: 6,
                    },
                    result: 32,
                },
            ],
            calculation_history_selection_index: Some(0),
            ..Default::default()
        };

        model.copy_selected_calculation_history();
        assert_eq!(model.expression_input_text, "0x21[5]");

        model.input_mode = InputMode::HistorySelect;
        model.calculation_history_selection_index = Some(1);
        model.copy_selected_calculation_history();
        assert_eq!(model.expression_input_text, "~0x5");
    }

    #[test]
    fn copy_selected_calculation_history_formats_literal_expressions() {
        let mut model = BitTwiddleModel {
            display_mode: DisplayMode::Hex,
            input_mode: InputMode::HistorySelect,
            calculation_history: vec![CalculationHistoryEntry {
                expression: ParsedExpression::Literal { value: 32 },
                result: 32,
            }],
            calculation_history_selection_index: Some(0),
            ..Default::default()
        };

        model.copy_selected_calculation_history();

        assert_eq!(model.expression_input_text, "0x20");
    }

    #[test]
    fn format_calculation_history_entry_includes_result() {
        let model = BitTwiddleModel {
            display_mode: DisplayMode::Hex,
            ..Default::default()
        };
        let entry = binary_history_entry(10, Operator::Add, 5, 15);

        assert_eq!(
            model.format_calculation_history_entry(&entry),
            "0xA + 0x5 = 0xF"
        );
    }
}
