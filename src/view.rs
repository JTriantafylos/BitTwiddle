use ratatui::{
    Frame,
    layout::{Constraint, Layout, Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Clear, List, ListItem, ListState, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState,
    },
};

use crate::{
    expression::IntegerWidth,
    model::{BitTwiddleModel, DisplayMode, InputMode},
};

const MIN_TERMINAL_WIDTH: u16 = 20;
const MIN_TERMINAL_HEIGHT: u16 = 7;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AppLayout {
    pub(crate) value_areas: Rect,
    pub(crate) mode_area: Option<Rect>,
    pub(crate) width_area: Option<Rect>,
    pub(crate) bit_area: Rect,
    pub(crate) input_area: Rect,
    pub(crate) expression_history_area: Option<Rect>,
    pub(crate) help_area: Option<Rect>,
    pub(crate) bits_per_line: u32,
}

pub(crate) fn app_layout(
    width: IntegerWidth,
    area: Rect,
    help_line_count: usize,
) -> Option<AppLayout> {
    if area.width < MIN_TERMINAL_WIDTH || area.height < MIN_TERMINAL_HEIGHT {
        return None;
    }

    let bits_per_line = bits_per_line_for_area(width, area.width);
    let bit_area_height = bit_line_count(width, bits_per_line) as u16 + 2;
    let required_core_height = 3 + bit_area_height + 3;
    if area.height < required_core_height {
        return None;
    }

    let remaining_height = area.height - required_core_height;
    let help_height = (help_line_count as u16).min(remaining_height);
    let history_height = remaining_height.saturating_sub(help_height);

    let mut y = area.y;
    let value_areas = Rect::new(area.x, y, area.width, 3);
    let (mode_area, width_area) = if value_areas.width >= 32 {
        let [_, mode_area, width_area] = Layout::horizontal([
            Constraint::Min(1),
            Constraint::Length(6),
            Constraint::Length(12),
        ])
        .areas(value_areas);
        (Some(mode_area), Some(width_area))
    } else {
        (None, None)
    };
    y += value_areas.height;
    let bit_area = Rect::new(area.x, y, area.width, bit_area_height);
    y += bit_area.height;
    let input_area = Rect::new(area.x, y, area.width, 3);
    y += input_area.height;
    let expression_history_area =
        (history_height > 0).then_some(Rect::new(area.x, y, area.width, history_height));
    y += history_height;
    let help_area = (help_height > 0).then_some(Rect::new(area.x, y, area.width, help_height));

    Some(AppLayout {
        value_areas,
        mode_area,
        width_area,
        bit_area,
        input_area,
        expression_history_area,
        help_area,
        bits_per_line,
    })
}

pub fn view(model: &BitTwiddleModel, frame: &mut Frame) {
    let Some(layout) = app_layout(
        model.integer_width,
        frame.area(),
        footer_help_line_count(model, frame.area().width),
    ) else {
        render_terminal_too_small(frame);
        return;
    };

    let value_areas = layout.value_areas;

    let val_text = Text::from(match &model.error {
        Some(err) => err.to_string(),
        None => model.format_value_readout(),
    });
    if value_areas.width >= 32 {
        let [value_area, mode_area, width_area] = Layout::horizontal([
            Constraint::Min(1),
            Constraint::Length(6),
            Constraint::Length(12),
        ])
        .areas(value_areas);
        frame.render_widget(
            Paragraph::new(val_text).block(Block::bordered()).centered(),
            value_area,
        );
        frame.render_widget(display_mode_indicator(&model.display_mode), mode_area);
        frame.render_widget(width_indicator(model.integer_width), width_area);
    } else {
        let title = format!(
            " {} {} ",
            display_mode_label(&model.display_mode),
            model.integer_width.bits()
        );
        frame.render_widget(
            Paragraph::new(val_text)
                .block(Block::bordered().title(title))
                .centered(),
            value_areas,
        );
    }

    let input_area = layout.input_area;
    let input_scroll = expression_input_scroll(model, input_area);
    let visible_input = &model.expression_input_text[input_scroll..];
    let input = Paragraph::new(visible_input).block(
        Block::bordered()
            .border_style(match model.input_mode {
                InputMode::ExpressionInput => Style::default().fg(Color::Yellow),
                _ => Style::default(),
            })
            .title("Input"),
    );
    frame.render_widget(input, input_area);
    if model.input_mode == InputMode::ExpressionInput {
        let cursor_offset = model
            .expression_input_cursor_index
            .saturating_sub(input_scroll)
            .min(input_area.width.saturating_sub(2).saturating_sub(1) as usize);
        frame.set_cursor_position(Position::new(
            input_area.x + cursor_offset as u16 + 1,
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
        layout.bits_per_line,
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
    frame.render_widget(bit_visualization, layout.bit_area);

    if let Some(expression_history_area) = layout.expression_history_area {
        let expression_history = List::new(
            model
                .calculation_history
                .iter()
                .rev()
                .map(|e| ListItem::new(model.format_calculation_history_entry(e))),
        )
        .block(
            Block::bordered()
                .border_style(match model.input_mode {
                    InputMode::HistorySelect => Style::default().fg(Color::Yellow),
                    _ => Style::default(),
                })
                .title(calculation_history_title(model)),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::REVERSED),
        );
        let mut expression_history_state = calculation_history_list_state(model);
        frame.render_stateful_widget(
            expression_history,
            expression_history_area,
            &mut expression_history_state,
        );
        if calculation_history_overflows(model, expression_history_area) {
            let mut scrollbar_state =
                calculation_history_scrollbar_state(model, expression_history_area);
            frame.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(None)
                    .end_symbol(None)
                    .track_style(Style::default().dim())
                    .thumb_style(Style::default().fg(Color::Yellow)),
                calculation_history_scrollbar_area(expression_history_area),
                &mut scrollbar_state,
            );
        }
    }

    if let Some(help_area) = layout.help_area {
        let help_text = footer_help_text(model, help_area.width);
        frame.render_widget(Paragraph::new(help_text).centered(), help_area);
    }

    if model.show_quit_confirmation {
        render_quit_confirmation(frame);
    }
}

fn render_terminal_too_small(frame: &mut Frame) {
    let text = Text::from(vec![
        Line::from("BitTwiddle"),
        Line::from("Resize terminal"),
    ]);
    frame.render_widget(Paragraph::new(text).centered(), frame.area());
}

fn display_mode_indicator(display_mode: &DisplayMode) -> Paragraph<'static> {
    let dec_indicator = Span::styled(
        "D",
        match display_mode {
            DisplayMode::Decimal => Style::default().underlined(),
            _ => Style::default().dim(),
        },
    );
    let hex_indicator = Span::styled(
        "H",
        match display_mode {
            DisplayMode::Hex => Style::default().underlined(),
            _ => Style::default().dim(),
        },
    );
    let signed_indicator = Span::styled(
        "S",
        match display_mode {
            DisplayMode::Signed => Style::default().underlined(),
            _ => Style::default().dim(),
        },
    );
    let oct_indicator = Span::styled(
        "O",
        match display_mode {
            DisplayMode::Octal => Style::default().underlined(),
            _ => Style::default().dim(),
        },
    );

    Paragraph::new(Line::from(vec![
        dec_indicator,
        signed_indicator,
        hex_indicator,
        oct_indicator,
    ]))
    .block(Block::bordered())
}

fn width_indicator(integer_width: IntegerWidth) -> Paragraph<'static> {
    Paragraph::new(Line::from(vec![
        width_span(integer_width, IntegerWidth::Eight, "8"),
        Span::raw(" "),
        width_span(integer_width, IntegerWidth::Sixteen, "16"),
        Span::raw(" "),
        width_span(integer_width, IntegerWidth::ThirtyTwo, "32"),
        Span::raw(" "),
        width_span(integer_width, IntegerWidth::SixtyFour, "64"),
    ]))
    .block(Block::bordered())
}

fn display_mode_label(display_mode: &DisplayMode) -> &'static str {
    match display_mode {
        DisplayMode::Decimal => "D",
        DisplayMode::Signed => "S",
        DisplayMode::Hex => "H",
        DisplayMode::Octal => "O",
    }
}

pub(crate) fn footer_help_text(model: &BitTwiddleModel, width: u16) -> Text<'static> {
    Text::from(
        footer_control_lines(&footer_controls(model), width)
            .into_iter()
            .map(footer_control_line)
            .collect::<Vec<_>>(),
    )
}

pub(crate) fn footer_help_line_count(model: &BitTwiddleModel, width: u16) -> usize {
    footer_control_lines(&footer_controls(model), width).len()
}

fn footer_controls(model: &BitTwiddleModel) -> Vec<(&'static str, &'static str)> {
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
                ("Middle-Click", "Paste"),
            ]);
        }
        InputMode::HistorySelect => {
            controls.extend([("Up/Down", "Select"), ("Enter", "Copy")]);
        }
    }

    if model.error.is_some() {
        controls.push(("Esc", "Dismiss"));
    }

    controls
}

fn footer_control_line(controls: Vec<(&'static str, &'static str)>) -> Line<'static> {
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

fn footer_control_lines(
    controls: &[(&'static str, &'static str)],
    width: u16,
) -> Vec<Vec<(&'static str, &'static str)>> {
    let width = width as usize;
    let mut lines = Vec::new();
    let mut current_line = Vec::new();

    for control in controls {
        let mut candidate = current_line.clone();
        candidate.push(*control);
        if !current_line.is_empty() && footer_controls_width(&candidate) > width {
            lines.push(current_line);
            current_line = vec![*control];
        } else {
            current_line = candidate;
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    lines
}

fn footer_controls_width(controls: &[(&str, &str)]) -> usize {
    controls
        .iter()
        .enumerate()
        .map(|(index, (key, label))| {
            let separator_width = if index > 0 { "  |  ".len() } else { 0 };
            separator_width + key.len() + 1 + label.len()
        })
        .sum()
}

pub(crate) fn calculation_history_list_state(model: &BitTwiddleModel) -> ListState {
    ListState::default().with_selected(
        (model.input_mode == InputMode::HistorySelect)
            .then_some(model.calculation_history_selection_index)
            .flatten(),
    )
}

pub(crate) fn calculation_history_title(model: &BitTwiddleModel) -> String {
    if model.input_mode == InputMode::HistorySelect
        && let Some(selection_index) = model.calculation_history_selection_index
        && selection_index < model.calculation_history.len()
    {
        return format!(
            "History {}/{}",
            selection_index + 1,
            model.calculation_history.len()
        );
    }

    "History".to_string()
}

pub(crate) fn calculation_history_overflows(model: &BitTwiddleModel, area: Rect) -> bool {
    model.calculation_history.len() > calculation_history_visible_rows(area)
}

fn calculation_history_visible_rows(area: Rect) -> usize {
    area.height.saturating_sub(2) as usize
}

pub(crate) fn calculation_history_scrollbar_area(area: Rect) -> Rect {
    Rect {
        x: area.x + area.width.saturating_sub(1),
        y: area.y.saturating_add(1),
        width: 1.min(area.width),
        height: area.height.saturating_sub(2),
    }
}

fn calculation_history_scrollbar_state(model: &BitTwiddleModel, area: Rect) -> ScrollbarState {
    ScrollbarState::new(model.calculation_history.len())
        .position(model.calculation_history_selection_index.unwrap_or(0))
        .viewport_content_length(calculation_history_visible_rows(area))
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
    bits_per_line: u32,
    display_value: u64,
    bit_manipulation_mode: bool,
    selected_bit_index: usize,
    selected_bit_style: Style,
) -> Vec<Line<'static>> {
    (0..bit_line_count(width, bits_per_line))
        .rev()
        .map(|line_num| {
            Line::from(
                bit_words_for_line(width, bits_per_line, line_num)
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

fn bits_per_line_for_area(width: IntegerWidth, area_width: u16) -> u32 {
    let inner_width = area_width.saturating_sub(2);
    [32, 16, 8, 4]
        .into_iter()
        .find(|bits_per_line| {
            (0..bit_line_count(width, *bits_per_line))
                .all(|line_num| bit_line_width(width, *bits_per_line, line_num) <= inner_width)
        })
        .unwrap_or(4)
}

pub(crate) fn bit_line_count(width: IntegerWidth, bits_per_line: u32) -> usize {
    width.bits().div_ceil(bits_per_line) as usize
}

fn bit_words_for_line(
    width: IntegerWidth,
    bits_per_line: u32,
    line_num: usize,
) -> impl DoubleEndedIterator<Item = (u32, u32)> {
    let line_start = line_num as u32 * bits_per_line;
    let line_end = ((line_num as u32 + 1) * bits_per_line).min(width.bits());
    (line_start..line_end).step_by(16).map(move |word_start| {
        let word_end = (word_start + 16).min(line_end);
        (word_start, word_end)
    })
}

pub(crate) fn hit_test_bit(
    width: IntegerWidth,
    bits_per_line: u32,
    area: Rect,
    column: u16,
    row: u16,
) -> Option<usize> {
    let inner_width = area.width.saturating_sub(2);
    let x = column.checked_sub(area.x + 1)?;
    let y = row.checked_sub(area.y + 1)?;

    let line_count = bit_line_count(width, bits_per_line);
    if y as usize >= line_count || x >= inner_width {
        return None;
    }

    let line_num = line_count - 1 - y as usize;
    let line_width = bit_line_width(width, bits_per_line, line_num);
    let line_start = inner_width.saturating_sub(line_width) / 2;
    let line_column = x.checked_sub(line_start)?;
    if line_column >= line_width {
        return None;
    }

    bit_index_at_line_column(width, bits_per_line, line_num, line_column)
}

pub(crate) fn expression_input_scroll(model: &BitTwiddleModel, area: Rect) -> usize {
    let inner_width = area.width.saturating_sub(2) as usize;
    model
        .expression_input_cursor_index
        .saturating_sub(inner_width.saturating_sub(1))
}

pub(crate) fn hit_test_input(area: Rect, scroll: usize, column: u16, row: u16) -> Option<usize> {
    let inner_width = area.width.saturating_sub(2);
    let x = column.checked_sub(area.x + 1)?;
    let y = row.checked_sub(area.y + 1)?;

    if y == 0 && x < inner_width {
        Some(scroll + x as usize)
    } else {
        None
    }
}

pub(crate) fn hit_test_value_area(area: Rect, column: u16, row: u16) -> bool {
    let x = column.checked_sub(area.x + 1).unwrap_or(u16::MAX);
    let y = row.checked_sub(area.y + 1).unwrap_or(u16::MAX);
    let inner_width = area.width.saturating_sub(2);
    let inner_height = area.height.saturating_sub(2);
    x < inner_width && y < inner_height
}

pub(crate) fn hit_test_display_mode(area: Rect, column: u16) -> Option<DisplayMode> {
    let x = column.checked_sub(area.x + 1)?;
    match x {
        0 => Some(DisplayMode::Decimal),
        1 => Some(DisplayMode::Signed),
        2 => Some(DisplayMode::Hex),
        3 => Some(DisplayMode::Octal),
        _ => None,
    }
}

pub(crate) fn hit_test_width(area: Rect, column: u16) -> Option<IntegerWidth> {
    let x = column.checked_sub(area.x + 1)?;
    match x {
        0 => Some(IntegerWidth::Eight),
        2 | 3 => Some(IntegerWidth::Sixteen),
        5 | 6 => Some(IntegerWidth::ThirtyTwo),
        8 | 9 => Some(IntegerWidth::SixtyFour),
        _ => None,
    }
}

pub(crate) fn hit_test_history(
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

pub(crate) fn bit_line_width(width: IntegerWidth, bits_per_line: u32, line_num: usize) -> u16 {
    let mut line_width = 0;
    for (word_start, word_end) in bit_words_for_line(width, bits_per_line, line_num) {
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
    bits_per_line: u32,
    line_num: usize,
    target_column: u16,
) -> Option<usize> {
    let mut column = 0;
    for (word_start, word_end) in bit_words_for_line(width, bits_per_line, line_num).rev() {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        expression::{Operator, ParsedExpression},
        model::{self, BitTwiddleModel},
    };
    use ratatui::{Terminal, backend::TestBackend, layout::Position};

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

    fn rendered_view_text(model: &BitTwiddleModel, width: u16, height: u16) -> String {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| view(model, frame)).unwrap();
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    fn rendered_view_symbol_at(
        model: &BitTwiddleModel,
        width: u16,
        height: u16,
        position: Position,
    ) -> String {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| view(model, frame)).unwrap();
        terminal
            .backend()
            .buffer()
            .cell(position)
            .unwrap()
            .symbol()
            .to_string()
    }

    fn hit_test_sixty_four_bit(area: Rect, column: u16, row: u16) -> Option<usize> {
        hit_test_bit(IntegerWidth::SixtyFour, 32, area, column, row)
    }

    #[test]
    fn calculation_history_state_tracks_selection_only_in_history_mode() {
        let mut model = BitTwiddleModel {
            input_mode: InputMode::HistorySelect,
            calculation_history_selection_index: Some(2),
            ..Default::default()
        };

        assert_eq!(calculation_history_list_state(&model).selected(), Some(2));

        model.input_mode = InputMode::ExpressionInput;
        assert_eq!(calculation_history_list_state(&model).selected(), None);
    }

    #[test]
    fn calculation_history_title_shows_position_only_for_active_selection() {
        let mut model = BitTwiddleModel {
            input_mode: InputMode::HistorySelect,
            calculation_history: (1..=23).map(history_entry).collect(),
            calculation_history_selection_index: Some(9),
            ..Default::default()
        };

        assert_eq!(calculation_history_title(&model), "History 10/23");

        model.input_mode = InputMode::ExpressionInput;
        assert_eq!(calculation_history_title(&model), "History");

        model.input_mode = InputMode::HistorySelect;
        model.calculation_history_selection_index = None;
        assert_eq!(calculation_history_title(&model), "History");
    }

    #[test]
    fn selected_history_entry_scrolls_into_visible_history_area() {
        let model = BitTwiddleModel {
            input_mode: InputMode::HistorySelect,
            calculation_history: (1..=10).map(history_entry).collect(),
            calculation_history_selection_index: Some(9),
            ..Default::default()
        };

        let rendered = rendered_view_text(&model, 82, 15);

        assert!(rendered.contains("1 + 1 = 2"));
        assert!(!rendered.contains("10 + 10 = 20"));
    }

    #[test]
    fn scrollbar_appears_only_when_history_overflows() {
        let frame_area = Rect::new(0, 0, 82, 17);
        let help_line_count = footer_help_line_count(&BitTwiddleModel::default(), frame_area.width);
        let history_area = app_layout(IntegerWidth::SixtyFour, frame_area, help_line_count)
            .unwrap()
            .expression_history_area
            .unwrap();
        let scrollbar_area = calculation_history_scrollbar_area(history_area);
        let scrollbar_position = Position::new(scrollbar_area.x, scrollbar_area.y);
        let overflowing = BitTwiddleModel {
            input_mode: InputMode::HistorySelect,
            calculation_history: (1..=10).map(history_entry).collect(),
            calculation_history_selection_index: Some(9),
            ..Default::default()
        };
        let fitting = BitTwiddleModel {
            input_mode: InputMode::HistorySelect,
            calculation_history: vec![history_entry(1)],
            calculation_history_selection_index: Some(0),
            ..Default::default()
        };

        assert!(calculation_history_overflows(&overflowing, history_area));
        assert!(!calculation_history_overflows(&fitting, history_area));
        assert_ne!(
            rendered_view_symbol_at(
                &overflowing,
                frame_area.width,
                frame_area.height,
                scrollbar_position
            ),
            rendered_view_symbol_at(
                &fitting,
                frame_area.width,
                frame_area.height,
                scrollbar_position
            )
        );
    }

    #[test]
    fn normal_size_uses_full_bit_rows_and_all_sections() {
        let layout = app_layout(IntegerWidth::SixtyFour, Rect::new(0, 0, 82, 14), 1).unwrap();

        assert_eq!(layout.value_areas, Rect::new(0, 0, 82, 3));
        assert_eq!(layout.bit_area, Rect::new(0, 3, 82, 4));
        assert_eq!(layout.input_area, Rect::new(0, 7, 82, 3));
        assert_eq!(layout.bits_per_line, 32);
        assert!(layout.expression_history_area.is_some());
        assert!(layout.help_area.is_some());
    }

    #[test]
    fn narrow_terminal_wraps_bits_to_shorter_rows() {
        let layout = app_layout(IntegerWidth::SixtyFour, Rect::new(0, 0, 40, 16), 1).unwrap();
        let rendered = rendered_view_text(&BitTwiddleModel::default(), 40, 16);

        assert_eq!(layout.bits_per_line, 8);
        assert_eq!(
            bit_line_count(IntegerWidth::SixtyFour, layout.bits_per_line),
            8
        );
        assert!(bit_line_width(IntegerWidth::SixtyFour, layout.bits_per_line, 7) <= 38);
        assert!(rendered.contains("Bits"));
        assert!(!rendered.contains("History"));
    }

    #[test]
    fn footer_help_text_wraps_controls_to_fit_width() {
        let model = BitTwiddleModel {
            input_mode: InputMode::ExpressionInput,
            ..Default::default()
        };
        let footer = footer_help_text(&model, 40);
        let lines = footer
            .lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>();

        assert_eq!(footer_help_line_count(&model, 40), 4);
        assert!(lines.iter().all(|line| line.len() <= 40));
        assert!(lines.join("\n").contains("Ctrl+C Clear"));
        assert!(lines.join("\n").contains("Left/Right Move"));
        assert!(lines.join("\n").contains("Middle-Click Paste"));
    }

    #[test]
    fn skinny_terminal_help_uses_multiple_lines_when_height_allows() {
        let layout = app_layout(
            IntegerWidth::SixtyFour,
            Rect::new(0, 0, 40, 19),
            footer_help_line_count(&BitTwiddleModel::default(), 40),
        )
        .unwrap();
        let rendered = rendered_view_text(&BitTwiddleModel::default(), 40, 19);

        assert_eq!(layout.help_area.unwrap().height, 3);
        assert!(rendered.contains("Tab Mode"));
        assert!(rendered.contains("Ctrl+C Clear"));
        assert!(rendered.contains("Left/Right Move"));
    }

    #[test]
    fn short_terminal_hides_history_before_core_controls() {
        let layout = app_layout(IntegerWidth::SixtyFour, Rect::new(0, 0, 82, 10), 1).unwrap();
        let rendered = rendered_view_text(&BitTwiddleModel::default(), 82, 10);

        assert_eq!(layout.bits_per_line, 32);
        assert_eq!(layout.expression_history_area, None);
        assert_eq!(layout.help_area, None);
        assert!(rendered.contains("Bits"));
        assert!(rendered.contains("Input"));
        assert!(!rendered.contains("History"));
    }

    #[test]
    fn too_small_terminal_shows_resize_message() {
        let rendered = rendered_view_text(&BitTwiddleModel::default(), 15, 8);

        assert!(rendered.contains("BitTwiddle"));
        assert!(rendered.contains("Resize terminal"));
        assert!(!rendered.contains("Input"));
    }

    #[test]
    fn bit_hit_test_maps_visible_top_row_bits() {
        let area = Rect::new(0, 0, 82, 4);

        assert_eq!(hit_test_sixty_four_bit(area, 5, 1), Some(63));
        assert_eq!(hit_test_sixty_four_bit(area, 7, 1), Some(62));
        assert_eq!(hit_test_sixty_four_bit(area, 11, 1), Some(60));
        assert_eq!(hit_test_sixty_four_bit(area, 14, 1), Some(59));
        assert_eq!(hit_test_sixty_four_bit(area, 78, 1), Some(32));
    }

    #[test]
    fn bit_hit_test_maps_visible_bottom_row_bits() {
        let area = Rect::new(0, 0, 82, 4);

        assert_eq!(hit_test_sixty_four_bit(area, 5, 2), Some(31));
        assert_eq!(hit_test_sixty_four_bit(area, 7, 2), Some(30));
        assert_eq!(hit_test_sixty_four_bit(area, 11, 2), Some(28));
        assert_eq!(hit_test_sixty_four_bit(area, 14, 2), Some(27));
        assert_eq!(hit_test_sixty_four_bit(area, 78, 2), Some(0));
    }

    #[test]
    fn bit_hit_test_ignores_labels_separators_borders_and_outside_area() {
        let area = Rect::new(0, 0, 82, 4);

        assert_eq!(hit_test_sixty_four_bit(area, 0, 1), None);
        assert_eq!(hit_test_sixty_four_bit(area, 1, 1), None);
        assert_eq!(hit_test_sixty_four_bit(area, 2, 1), None);
        assert_eq!(hit_test_sixty_four_bit(area, 3, 1), None);
        assert_eq!(hit_test_sixty_four_bit(area, 6, 1), None);
        assert_eq!(hit_test_sixty_four_bit(area, 82, 1), None);
        assert_eq!(hit_test_sixty_four_bit(area, 5, 0), None);
        assert_eq!(hit_test_sixty_four_bit(area, 5, 3), None);
    }

    #[test]
    fn bit_hit_test_uses_active_integer_width() {
        let area = Rect::new(0, 0, 82, 4);

        assert_eq!(hit_test_bit(IntegerWidth::Eight, 32, area, 33, 1), Some(7));
        assert_eq!(hit_test_bit(IntegerWidth::Eight, 32, area, 48, 1), Some(0));
        assert_eq!(hit_test_bit(IntegerWidth::Eight, 32, area, 5, 1), None);
        assert_eq!(hit_test_bit(IntegerWidth::Eight, 32, area, 34, 2), None);
    }

    #[test]
    fn input_hit_test_ignores_borders_and_outside_area() {
        let area = Rect::new(0, 7, 82, 3);

        assert_eq!(hit_test_input(area, 0, 1, 8), Some(0));
        assert_eq!(hit_test_input(area, 0, 0, 8), None);
        assert_eq!(hit_test_input(area, 0, 81, 8), None);
        assert_eq!(hit_test_input(area, 0, 1, 7), None);
        assert_eq!(hit_test_input(area, 0, 1, 9), None);
    }

    #[test]
    fn input_hit_test_accounts_for_horizontal_scroll() {
        let area = Rect::new(0, 7, 10, 3);

        assert_eq!(hit_test_input(area, 5, 1, 8), Some(5));
        assert_eq!(hit_test_input(area, 5, 8, 8), Some(12));
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
    fn normal_layout_includes_mode_and_width_areas() {
        let layout = app_layout(IntegerWidth::SixtyFour, Rect::new(0, 0, 82, 14), 1).unwrap();

        assert!(layout.mode_area.is_some());
        assert!(layout.width_area.is_some());
        assert_eq!(layout.mode_area.unwrap().width, 6);
        assert_eq!(layout.width_area.unwrap().width, 12);
    }

    #[test]
    fn narrow_layout_has_no_mode_or_width_areas() {
        let layout = app_layout(IntegerWidth::SixtyFour, Rect::new(0, 0, 31, 16), 1).unwrap();

        assert!(layout.mode_area.is_none());
        assert!(layout.width_area.is_none());
    }

    #[test]
    fn hit_test_display_mode_maps_columns_to_modes() {
        let area = Rect::new(0, 0, 82, 3);

        assert_eq!(hit_test_display_mode(area, 1), Some(DisplayMode::Decimal));
        assert_eq!(hit_test_display_mode(area, 2), Some(DisplayMode::Signed));
        assert_eq!(hit_test_display_mode(area, 3), Some(DisplayMode::Hex));
        assert_eq!(hit_test_display_mode(area, 4), Some(DisplayMode::Octal));
    }

    #[test]
    fn hit_test_display_mode_ignores_borders_and_outside_area() {
        let area = Rect::new(0, 0, 82, 3);

        assert_eq!(hit_test_display_mode(area, 0), None);
        assert_eq!(hit_test_display_mode(area, 5), None);
        assert_eq!(hit_test_display_mode(area, 81), None);
    }

    #[test]
    fn hit_test_width_maps_columns_to_widths() {
        let area = Rect::new(0, 0, 82, 3);

        assert_eq!(hit_test_width(area, 1), Some(IntegerWidth::Eight));
        assert_eq!(hit_test_width(area, 3), Some(IntegerWidth::Sixteen));
        assert_eq!(hit_test_width(area, 4), Some(IntegerWidth::Sixteen));
        assert_eq!(hit_test_width(area, 6), Some(IntegerWidth::ThirtyTwo));
        assert_eq!(hit_test_width(area, 7), Some(IntegerWidth::ThirtyTwo));
        assert_eq!(hit_test_width(area, 9), Some(IntegerWidth::SixtyFour));
        assert_eq!(hit_test_width(area, 10), Some(IntegerWidth::SixtyFour));
    }

    #[test]
    fn hit_test_width_ignores_borders_and_outside_area() {
        let area = Rect::new(0, 0, 82, 3);

        assert_eq!(hit_test_width(area, 0), None);
        assert_eq!(hit_test_width(area, 2), None);
        assert_eq!(hit_test_width(area, 5), None);
        assert_eq!(hit_test_width(area, 8), None);
        assert_eq!(hit_test_width(area, 11), None);
        assert_eq!(hit_test_width(area, 81), None);
    }

    #[test]
    fn hit_test_value_area_detects_clicks_inside_value_display() {
        let area = Rect::new(0, 0, 82, 3);

        assert!(hit_test_value_area(area, 1, 1));
        assert!(hit_test_value_area(area, 40, 1));
        assert!(hit_test_value_area(area, 80, 1));
    }

    #[test]
    fn hit_test_value_area_ignores_borders_and_outside_area() {
        let area = Rect::new(0, 0, 82, 3);

        // Top and bottom borders
        assert!(!hit_test_value_area(area, 1, 0));
        assert!(!hit_test_value_area(area, 1, 2));
        // Outside vertical range
        assert!(!hit_test_value_area(area, 1, 3));
        // Left and right borders
        assert!(!hit_test_value_area(area, 0, 1));
        assert!(!hit_test_value_area(area, 81, 1));
    }
}
