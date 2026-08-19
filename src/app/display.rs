use super::state::{App, SlotKind, SlotState};
use crate::models::AppState;

const NEWLINE_GLYPH: char = '↵'; // u+23CE
const TAB_GLYPH: char = '·'; // u+00B7

impl App {
    pub fn resize(&mut self, width: u16, _height: u16) {
        self.terminal_width = width;
        self.recalculate_lines();
    }

    pub fn on_mouse(&mut self) {
        if self.test.state != AppState::Finished {
            self.show_ui = true;
        }
    }

    pub(crate) fn sync_display_text(&mut self) {
        let is_code = self.config.word_data.name.starts_with("code_");
        let cap = self.test.slots.len() + 20;
        let mut new_display = String::with_capacity(cap);
        let mut new_mask    = Vec::<u8>::with_capacity(cap);
        let mut new_aligned = Vec::<char>::with_capacity(cap);
        let mut new_breaks  = Vec::<bool>::with_capacity(cap);
        let mut display_pos: usize = 0;
        let mut first_pending_display_pos: Option<usize> = None;
        let mut deferred_break: Option<usize> = None;

        let render_up_to = if is_code {
            self.test.slots.iter()
                .rposition(|s| s.kind != SlotKind::Newline)
                .map(|i| i + 1)
                .unwrap_or(0)
        } else {
            self.test.slots.len()
        };

        for slot in self.test.slots[..render_up_to].iter() {
            let (glyph, glyph_count) = match slot.kind {
                SlotKind::Newline if is_code => (NEWLINE_GLYPH, 1),
                SlotKind::Tab     if is_code => (TAB_GLYPH, slot.visual_width as usize),
                _                            => (slot.expected, 1),
            };

            let is_visual_only = is_code &&
                matches!(slot.kind, SlotKind::Newline | SlotKind::Tab);

            if is_code && slot.kind == SlotKind::Newline {
                if let Some(pos) = deferred_break.take() {
                    new_breaks[pos] = true;
                }
                deferred_break = Some(display_pos + glyph_count - 1);
            } else if matches!(slot.state, SlotState::Extra(_)) && deferred_break.is_some() {
                deferred_break = Some(display_pos);
            } else if let Some(pos) = deferred_break.take() {
                new_breaks[pos] = true;
            }

            match &slot.state {
                SlotState::Pending => {
                    if first_pending_display_pos.is_none() {
                        first_pending_display_pos = Some(display_pos);
                    }
                    for _ in 0..glyph_count {
                        new_display.push(glyph);
                        new_mask.push(0);
                    }
                }

                SlotState::Correct => {
                    for _ in 0..glyph_count {
                        new_display.push(glyph);
                        new_mask.push(if is_visual_only { 4 } else { 0 });
                        if !is_visual_only {
                            new_aligned.push(glyph);
                        }
                    }
                }

                SlotState::Wrong(typed) => {
                    for _ in 0..glyph_count {
                        new_display.push(glyph);
                        new_mask.push(1);
                        if !is_visual_only {
                            new_aligned.push(*typed);
                        }
                    }
                }

                SlotState::Uncommitted(typed) => {
                    for _ in 0..glyph_count {
                        new_display.push(glyph);
                        new_mask.push(1);
                        if !is_visual_only {
                            new_aligned.push(*typed);
                        }
                    }
                }

                SlotState::Extra(typed) => {
                    let display_glyph = if is_code && *typed == '\n' { NEWLINE_GLYPH } else { *typed };
                    new_display.push(display_glyph);
                    new_mask.push(2);
                    new_aligned.push(display_glyph);
                }

                SlotState::Missed => {
                    for _ in 0..glyph_count {
                        new_display.push(glyph);
                        new_mask.push(if is_visual_only || glyph == ' ' { 3 } else { 0 });
                        if !is_visual_only {
                            new_aligned.push('\0');
                        }
                    }
                }
            }

            for _ in 0..glyph_count {
                new_breaks.push(false);
            }
            display_pos += glyph_count;
        }

        self.test.display_string   = new_display;
        self.test.display_mask     = new_mask;
        self.test.aligned_input    = new_aligned;
        self.test.display_breaks   = new_breaks;
        self.test.extra_char_count = self.test.display_mask.iter().filter(|&&x| x == 2).count();
        self.test.cursor_idx = first_pending_display_pos.unwrap_or(display_pos);
        self.recalculate_lines();
    }

    pub(crate) fn will_cause_visual_wrap(&self, extra_char: char, is_extra: bool) -> bool {
        let layout_width = (self.terminal_width as usize * 80) / 100;
        let candidate_width = if is_extra { layout_width } else { layout_width.saturating_sub(2) };

        let current_line_idx = Self::line_idx_for_cursor(
            &self.test.visual_lines,
            self.test.aligned_input.len(),
        );

        let mut candidate_display = self.test.display_string.clone();
        candidate_display.push(extra_char);
        let mut candidate_breaks = self.test.display_breaks.clone();
        candidate_breaks.push(false);
        let candidate_lines = Self::wrap_into_lines(&candidate_display, candidate_width, &candidate_breaks);

        let candidate_line_idx = Self::line_idx_for_cursor(
            &candidate_lines,
            self.test.aligned_input.len() + 1,
        );

        if is_extra {
            candidate_line_idx > current_line_idx
        } else {
            candidate_line_idx >= 3
        }
    }

    pub(crate) fn wrap_into_lines(text: &str, width: usize, breaks: &[bool]) -> Vec<String> {
        let is_code = text.contains(NEWLINE_GLYPH);
        if is_code {
            Self::wrap_into_lines_code(text, width, breaks)
        } else {
            Self::wrap_into_lines_normal(text, width)
        }
    }

    fn wrap_into_lines_code(text: &str, width: usize, breaks: &[bool]) -> Vec<String> {
        let mut lines: Vec<String> = Vec::new();
        let mut current_line = String::new();
        let mut current_width = 0usize;

        for (idx, c) in text.chars().enumerate() {
            let hard_break = breaks.get(idx).copied().unwrap_or(false);
            if hard_break {
                current_line.push(c);
                lines.push(current_line.clone());
                current_line.clear();
                current_width = 0;
            } else if current_width >= width {
                if let Some(last_space) = current_line.rfind(' ') {
                    let next = current_line[last_space + 1..].to_string();
                    current_line.truncate(last_space + 1);
                    lines.push(current_line.clone());
                    current_line = next;
                    current_width = current_line.chars().count();
                } else {
                    lines.push(current_line.clone());
                    current_line.clear();
                    current_width = 0;
                }
                current_line.push(c);
                current_width += 1;
            } else {
                current_line.push(c);
                current_width += 1;
            }
        }

        if !current_line.is_empty() {
            lines.push(current_line);
        }
        lines
    }

    fn wrap_into_lines_normal(text: &str, width: usize) -> Vec<String> {
        let mut lines: Vec<String> = Vec::new();
        let mut current_line  = String::new();
        let mut current_width = 0usize;

        for word in text.split(' ') {
            let word_len     = word.chars().count();
            let space_before = if current_width == 0 { 0 } else { 1 };

            if current_width + space_before + word_len <= width {
                if current_width > 0 {
                    current_line.push(' ');
                    current_width += 1;
                }
                current_line.push_str(word);
                current_width += word_len;
            } else {
                if !current_line.is_empty() {
                    lines.push(current_line.clone());
                }
                current_line  = word.to_string();
                current_width = word_len;
            }
        }

        if !current_line.is_empty() {
            lines.push(current_line);
        }
        lines
    }

    pub(crate) fn line_idx_for_cursor(lines: &[String], cursor_pos: usize) -> usize {
        let mut running = 0usize;
        for (i, line) in lines.iter().enumerate() {
            let line_len = line.chars().count() + 1;
            if cursor_pos < running + line_len { return i; }
            running += line_len;
        }
        if lines.is_empty() { 0 } else { lines.len() - 1 }
    }

    pub(crate) fn recalculate_lines(&mut self) {
        let layout_width = (self.terminal_width as usize * 80) / 100;
        let safe_width   = layout_width.saturating_sub(2);
        self.test.visual_lines = Self::wrap_into_lines(&self.test.display_string, safe_width, &self.test.display_breaks);
    }
}
