use crate::app::App;
use crate::models::Mode;
use crate::models::AppState;
use crate::app::state::{SlotKind, SlotState};
use crate::ui::utils::{format_timer, hex_to_rgb, render_header, render_footer};
use crate::utils::strings;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub fn draw(f: &mut Frame, app: &App) {
    let status_text = match app.config.mode {
        Mode::Time(limit) => {
            let seconds = if let Some(start) = app.test.start_time {
                let elapsed = start.elapsed().as_secs();
                limit.saturating_sub(elapsed)
            } else {
                limit
            };
            format_timer(seconds)
        }
        Mode::Words(total) => {
            let visible_words = app.test.input.split_whitespace().count();
            let mut total_typed = app.test.scrolled_word_count + visible_words;
            let is_finished = app.test.aligned_input.len() >= app.test.word_stream_string.chars().count();
            if !app.test.input.ends_with(' ') && !is_finished && visible_words > 0 {
                total_typed = total_typed.saturating_sub(1);
            }
            format!("{}/{}", total_typed, total)
        }
        Mode::Quote(_) => {
            let is_code = app.config.word_data.name.starts_with("code_");
            if is_code {
                format!("{}/{}", app.code_typed_word_count(), app.test.total_code_words)
            } else {
                let visible_words = app.test.input.split_whitespace().count();
                let mut typed_words = app.test.scrolled_word_count + visible_words;
                let is_finished =
                    app.test.aligned_input.len() >= app.test.word_stream_string.chars().count() && app.test.quote_pool.is_empty();
                if !app.test.input.ends_with(' ') && !is_finished && visible_words > 0 {
                    typed_words = typed_words.saturating_sub(1);
                }
                format!("{}/{}", typed_words, app.test.total_quote_words)
            }
        }
    };

    render_header(f, app);

    let vertical_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(6),
            Constraint::Fill(1),
        ])
        .split(f.area());

    let horizontal_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),
            Constraint::Percentage(80),
            Constraint::Fill(1),
        ])
        .split(vertical_layout[1]);

    let active_area = horizontal_layout[1];
    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(0),
            Constraint::Min(1),
        ])
        .split(active_area);

    f.render_widget(
        Paragraph::new(status_text)
            .alignment(Alignment::Left)
            .style(
                Style::default()
                    .fg(hex_to_rgb(&app.config.theme.main))
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
        inner_chunks[0],
    );

    let elapsed_ms = app.test.caret_epoch.elapsed().as_millis();
    const BLINK_PERIOD_MS: u128 = 530;

    let caret_visible = app.test.state == AppState::Running
        || (elapsed_ms / BLINK_PERIOD_MS) % 2 == 0;

    let mut visible_lines: Vec<Line> = Vec::new();
    let lines_to_show = app.test.visual_lines.iter().take(3);

    let mut global_char_idx = 0usize;
    let input_chars = &app.test.aligned_input;
    let text_area = inner_chunks[2];

    let is_code = app.config.word_data.name.starts_with("code_");

    let caret_pos = app.test.cursor_idx;

    let uncommitted_glyph = if is_code { app.uncommitted_glyph() } else { None };
    let caret_on_next_line = uncommitted_glyph.is_some();

    let (display_to_aligned, display_typed): (Vec<Option<usize>>, Vec<bool>) = if is_code {
        let mut map = Vec::with_capacity(app.test.display_string.chars().count());
        let mut typed_flags = Vec::with_capacity(app.test.display_string.chars().count());
        let mut aligned_idx = 0usize;
        for slot in &app.test.slots {
            let is_visual_only = matches!(slot.kind, SlotKind::Newline | SlotKind::Tab);
            let glyph_count = slot.visual_width as usize;
            let typed = matches!(
                slot.state,
                SlotState::Correct
                    | SlotState::Wrong(_)
                    | SlotState::Uncommitted(_)
                    | SlotState::Extra(_)
            );
            let consumes_aligned = !is_visual_only
                && matches!(
                    slot.state,
                    SlotState::Correct
                        | SlotState::Wrong(_)
                        | SlotState::Uncommitted(_)
                        | SlotState::Extra(_)
                        | SlotState::Missed
                );
            for _ in 0..glyph_count {
                if is_visual_only {
                    map.push(None);
                } else {
                    map.push(Some(aligned_idx));
                }
                typed_flags.push(!is_visual_only && typed);
            }
            if consumes_aligned {
                aligned_idx += 1;
            }
        }
        (map, typed_flags)
    } else {
        (Vec::new(), Vec::new())
    };

    let color_correct = hex_to_rgb(&app.config.theme.text);
    let color_incorrect = hex_to_rgb(&app.config.theme.error);
    let color_future = hex_to_rgb(&app.config.theme.sub);

    let color_cursor_bg = hex_to_rgb(&app.config.theme.caret);
    let color_cursor_fg = hex_to_rgb(&app.config.theme.sub);

    for line_str in lines_to_show {
        let mut spans: Vec<Span> = Vec::new();

        for (char_idx, c) in line_str.chars().enumerate() {
            let current_idx = global_char_idx + char_idx;

            let aligned_idx_opt: Option<usize> = if is_code {
                display_to_aligned.get(current_idx).copied().flatten()
            } else {
                Some(current_idx)
            };

            let is_visual_only_pos = is_code && aligned_idx_opt.is_none();

            let is_extra_char = if current_idx < app.test.display_mask.len() {
                app.test.display_mask[current_idx] != 0
            } else {
                false
            };

            let is_typed = if is_code {
                display_typed.get(current_idx).copied().unwrap_or(false)
            } else {
                match aligned_idx_opt {
                    Some(ai) => ai < input_chars.len(),
                    None => false,
                }
            };

            if current_idx == caret_pos && !caret_on_next_line {
                spans.push(Span::styled(
                    c.to_string(),
                    if caret_visible {
                        Style::default().bg(color_cursor_bg).fg(color_cursor_fg)
                    } else {
                        Style::default().fg(color_future)
                    },
                ));
            } else if is_visual_only_pos {
                let mask_val = app.test.display_mask.get(current_idx).copied().unwrap_or(0);
                if mask_val == 1 {
                    spans.push(Span::styled(
                        c.to_string(),
                        Style::default()
                            .fg(color_incorrect)
                            .add_modifier(ratatui::style::Modifier::BOLD),
                    ));
                } else if mask_val == 3 {
                    spans.push(Span::styled(c.to_string(), Style::default().fg(color_future)));
                } else {
                    let is_at_or_ahead_of_caret = current_idx >= caret_pos;
                    if is_at_or_ahead_of_caret {
                        spans.push(Span::styled(c.to_string(), Style::default().fg(color_future)));
                    } else {
                        spans.push(Span::styled(
                            c.to_string(),
                            Style::default()
                                .fg(color_correct)
                                .add_modifier(ratatui::style::Modifier::BOLD),
                        ));
                    }
                }
            } else if is_typed {
                let ai = aligned_idx_opt.unwrap();
                if is_extra_char {
                    spans.push(Span::styled(
                        c.to_string(),
                        Style::default()
                            .fg(color_incorrect)
                            .add_modifier(ratatui::style::Modifier::BOLD),
                    ));
                } else {
                    let input_char = input_chars[ai];
                    if input_char == '\0' {
                        spans.push(Span::styled(c.to_string(), Style::default().fg(color_future)));
                    } else if strings::are_characters_visually_equal(input_char, c) {
                        spans.push(Span::styled(
                            c.to_string(),
                            Style::default()
                                .fg(color_correct)
                                .add_modifier(ratatui::style::Modifier::BOLD),
                        ));
                    } else {
                        spans.push(Span::styled(
                            c.to_string(),
                            Style::default()
                                .fg(color_incorrect)
                                .add_modifier(ratatui::style::Modifier::BOLD),
                        ));
                    }
                }
            } else {
                spans.push(Span::styled(c.to_string(), Style::default().fg(color_future)));
            }
        }

        let line_end_idx = global_char_idx + line_str.chars().count();
        let caret_at_uncommitted_end = is_code
            && caret_on_next_line
            && caret_pos == line_end_idx;
        if caret_visible && ((!is_code && caret_pos == line_end_idx) || caret_at_uncommitted_end) {
            spans.push(Span::styled(
                " ",
                Style::default().bg(color_cursor_bg),
            ));
        }

        let line_separator = if is_code { 0 } else { 1 };
        global_char_idx += line_str.chars().count() + line_separator;
        visible_lines.push(Line::from(spans));
    }

    f.render_widget(
        Paragraph::new(visible_lines).alignment(Alignment::Left),
        text_area,
    );

    render_footer(f, app);
}
