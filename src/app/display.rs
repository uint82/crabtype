use super::state::App;
use crate::models::AppState;

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
        let clean_chars: Vec<char> = self.test.word_stream_string.chars().collect();
        let input_chars: Vec<char> = self.test.input.chars().collect();

        let mut new_display = String::with_capacity(self.test.word_stream_string.len() + 20);
        let mut new_mask: Vec<bool>   = Vec::with_capacity(self.test.word_stream_string.len() + 20);
        let mut new_aligned: Vec<char> = Vec::with_capacity(self.test.word_stream_string.len() + 20);

        let mut clean_idx = 0;
        let mut input_idx = 0;
        let mut word_idx  = 0usize;

        while clean_idx < clean_chars.len() {
            let clean_char = clean_chars[clean_idx];
            if clean_char == ' ' {
                while input_idx < input_chars.len() && input_chars[input_idx] != ' ' {
                    new_display.push(input_chars[input_idx]);
                    new_mask.push(true);
                    new_aligned.push(input_chars[input_idx]);
                    input_idx += 1;
                }
                if input_idx < input_chars.len() && input_chars[input_idx] == ' ' {
                    // inject \0 slots so aligned_input has the right length for missed positions
                    if let Some(&missed) = self.test.missed_chars.get(&word_idx) {
                        for _ in 0..missed {
                            new_aligned.push('\0');
                        }
                    }
                    new_display.push(' ');
                    new_mask.push(false);
                    new_aligned.push(' ');
                    input_idx += 1;
                    word_idx  += 1;
                } else {
                    new_display.push(' ');
                    new_mask.push(false);
                }
                clean_idx += 1;
            } else {
                new_display.push(clean_char);
                new_mask.push(false);
                clean_idx += 1;
                if input_idx < input_chars.len() && input_chars[input_idx] != ' ' {
                    new_aligned.push(input_chars[input_idx]);
                    input_idx += 1;
                }
            }
        }

        self.test.display_string  = new_display;
        self.test.display_mask    = new_mask;
        self.test.aligned_input   = new_aligned;
        self.test.extra_char_count = self.test.display_mask.iter().filter(|&&x| x).count();
        self.test.cursor_idx = self.test.aligned_input.len();
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
        let candidate_lines = Self::wrap_into_lines(&candidate_display, candidate_width);

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

    pub(crate) fn wrap_into_lines(text: &str, width: usize) -> Vec<String> {
        let mut lines: Vec<String> = Vec::new();
        let mut current_line = String::new();
        let mut current_width = 0usize;

        for word in text.split(' ') {
            let word_len    = word.chars().count();
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
                current_line.clear();
                current_line.push_str(word);
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
        self.test.visual_lines = Self::wrap_into_lines(&self.test.display_string, safe_width);
    }
}
