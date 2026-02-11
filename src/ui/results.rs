use crate::app::App;
use crate::models::{Mode, QuoteSelector};
use crate::ui::utils::{hex_to_rgb, get_quote_length_category, render_header, render_footer};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Paragraph},
    Frame,
};

pub fn draw(f: &mut Frame, app: &App) {
    render_header(f, app);

    let main_area = Rect::new(
        0,
        2,
        f.area().width,
        f.area().height.saturating_sub(3),
    );

    let vertical_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(12),
            Constraint::Fill(1),
        ])
        .split(main_area);

    let horizontal_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),
            Constraint::Percentage(80),
            Constraint::Fill(1),
        ])
        .split(vertical_chunks[1]);

    let area = horizontal_layout[1];

    let inner_layout = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .split(area);

    let sub_color = hex_to_rgb(&app.theme.sub);
    let main_color = hex_to_rgb(&app.theme.main);

    let wpm_line = Line::from(vec![
        Span::styled("wpm: ", Style::default().fg(sub_color)),
        Span::styled(
            format!("{:.0}", app.final_wpm),
            Style::default()
                .fg(main_color)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ),
    ]);

    f.render_widget(
        Paragraph::new(wpm_line).alignment(Alignment::Center),
        inner_layout[0],
    );

    let stats_line = Line::from(vec![
        Span::styled("acc: ", Style::default().fg(sub_color)),
        Span::styled(
            format!("{:.2}%", app.final_accuracy),
            Style::default().fg(main_color),
        ),
        Span::styled(" | raw: ", Style::default().fg(sub_color)),
        Span::styled(
            format!("{:.0}", app.final_raw_wpm),
            Style::default().fg(main_color),
        ),
    ]);

    f.render_widget(
        Paragraph::new(stats_line).alignment(Alignment::Center),
        inner_layout[1],
    );

    let (_, _, vis_raw_cor, vis_raw_inc, vis_raw_ext, vis_raw_mis) =
        app.calculate_custom_stats_for_slice(&app.input, &app.display_string, &app.display_mask);

    let total_raw_cor = app.st_correct + vis_raw_cor;
    let total_raw_inc = app.st_incorrect + vis_raw_inc;
    let total_raw_ext = app.st_extra + vis_raw_ext;
    let total_raw_mis = app.st_missed + vis_raw_mis;

    let chars_line = Line::from(vec![
        Span::styled("cor: ", Style::default().fg(sub_color)),
        Span::styled(format!("{}", total_raw_cor), Style::default().fg(main_color)),
        Span::styled(" | inc: ", Style::default().fg(sub_color)),
        Span::styled(format!("{}", total_raw_inc), Style::default().fg(main_color)),
        Span::styled(" | ext: ", Style::default().fg(sub_color)),
        Span::styled(format!("{}", total_raw_ext), Style::default().fg(main_color)),
        Span::styled(" | mis: ", Style::default().fg(sub_color)),
        Span::styled(format!("{}", total_raw_mis), Style::default().fg(main_color)),
        Span::styled(" | time: ", Style::default().fg(sub_color)),
        Span::styled(
            format!("{:.1}s", app.final_time),
            Style::default().fg(main_color),
        ),
    ]);

    f.render_widget(
        Paragraph::new(chars_line).alignment(Alignment::Center),
        inner_layout[2],
    );

    let total_keystrokes = app.live_correct_keystrokes + app.live_incorrect_keystrokes;
    let debug_line = Line::from(vec![
        Span::styled("Accuracy: (", Style::default().fg(sub_color)),
        Span::styled("correct = ", Style::default().fg(sub_color)),
        Span::styled(format!("{}", app.live_correct_keystrokes), Style::default().fg(main_color)),
        Span::styled(", incorrect = ", Style::default().fg(sub_color)),
        Span::styled(format!("{}", app.live_incorrect_keystrokes), Style::default().fg(main_color)),
        Span::styled(", total = ", Style::default().fg(sub_color)),
        Span::styled(format!("{}", total_keystrokes), Style::default().fg(main_color)),
        Span::styled(")", Style::default().fg(sub_color)),
        Span::styled(" = ", Style::default().fg(sub_color)),
        Span::styled(
            format!("{:.2}%", app.final_accuracy),
            Style::default().fg(main_color),
        ),
    ]);

    f.render_widget(
        Paragraph::new(debug_line).alignment(Alignment::Center),
        inner_layout[3],
    );

    let mode_str = match &app.mode {
        Mode::Time(t) => format!("time {}", t),
        Mode::Words(w) => format!("word {}", w),
        Mode::Quote(q) => match q {
            QuoteSelector::Id(_) => {
                let actual_length = get_quote_length_category(app.original_quote_length);
                format!("quote {}", actual_length)
            },
            QuoteSelector::Category(len) => {
                let len_str = format!("{:?}", len).to_lowercase();
                let actual_length = if len_str == "all" {
                    get_quote_length_category(app.original_quote_length)
                } else {
                    &len_str
                };
                format!("quote {}", actual_length)
            },
        },
    };
    let mut type_parts = vec![mode_str, app.word_data.name.clone()];
    if app.use_punctuation {
        type_parts.push("punctuation".to_string());
    }
    if app.use_numbers {
        type_parts.push("number".to_string());
    }

    let type_value = type_parts.join(" ");

    let type_line = Line::from(vec![
        Span::styled("test type: ", Style::default().fg(sub_color)),
        Span::styled(type_value, Style::default().fg(main_color)),
    ]);

    f.render_widget(
        Paragraph::new(type_line).alignment(Alignment::Center),
        inner_layout[4],
    );

    if !app.current_quote_source.is_empty() {
        let source_line = Line::from(vec![
            Span::styled("source: ", Style::default().fg(sub_color)),
            Span::styled(&app.current_quote_source, Style::default().fg(main_color)),
        ]);

        f.render_widget(
            Paragraph::new(source_line).alignment(Alignment::Center),
            inner_layout[6],
        );
    }

    render_footer(f, app);
}
