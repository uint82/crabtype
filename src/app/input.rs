use super::state::App;
use crate::models::{AppState, Mode, WordState};
use crate::utils::strings;
use std::time::Instant;

impl App {
    pub fn on_key(&mut self, c: char) {
        if self.test.state == AppState::Finished { return; }
        if self.test.state == AppState::Waiting {
            self.test.start_time = Some(Instant::now());
            self.test.state = AppState::Running;
            if let Some(ref mut d) = self.discord {
                use crate::ui::utils::quote_idle_label;
                let ql = match &self.config.mode {
                    Mode::Quote(q) => quote_idle_label(q, self.test.original_quote_length),
                    _ => "",
                };
                d.set_typing(&self.config.mode, self.config.use_punctuation, self.config.use_numbers, ql, &self.config.word_data.name);
            }
        }

        self.record_snapshot_if_needed();

        let current_input_segments: Vec<&str> = self.test.input.split(' ').collect();
        let word_idx = current_input_segments.len().saturating_sub(1);

            if word_idx < self.test.word_stream.len() {
                let target_word_struct = &self.test.word_stream[word_idx];
                let target_word = &target_word_struct.text;
                let user_current_word = current_input_segments.last().unwrap_or(&"");

                if c == ' ' && user_current_word.is_empty() { return; }

            let target_char_count = target_word.chars().count();
            let user_char_count = user_current_word.chars().count();

            // use char count for the limit, byte len is wrong for multi-byte chars like em dash
            let limit = target_char_count + 19;
            if user_char_count >= limit {
                if c != ' ' { return; }
            }

            if c != ' ' {
                let is_extra = user_char_count >= target_char_count;
                if self.will_cause_visual_wrap(c, is_extra) { return; }

                let is_finite_mode = matches!(self.config.mode, Mode::Words(_) | Mode::Quote(_));
                if is_finite_mode {
                    let last_word_idx = self.test.word_stream_string
                        .split(' ')
                        .count()
                        .saturating_sub(1);
                    if word_idx >= last_word_idx && user_char_count >= target_char_count {
                        return;
                    }
                }
            }
        }

        self.show_ui = false;
        self.test.gross_char_count += 1;

        // compare relative to the current word. global indices break when extra chars shift positions
        let is_keystroke_correct = if word_idx < self.test.word_stream.len() {
            let target_word = &self.test.word_stream[word_idx].text;
            let user_current_word = current_input_segments.last().unwrap_or(&"");

            if c == ' ' {
                // word-level visual equality so hyphens typed against em-dash or en-dash counts as correct
                Self::words_visually_equal(user_current_word, target_word)
            } else {
                let user_char_count = user_current_word.chars().count();
                let target_char_count = target_word.chars().count();
                if user_char_count < target_char_count {
                    // use char index, not byte index. target_word may contain multi-byte chars
                    let target_char = target_word.chars().nth(user_char_count).unwrap_or('\0');
                    strings::are_characters_visually_equal(c, target_char)
                } else {
                    false
                }
            }
        } else {
            false
        };

        if is_keystroke_correct {
            self.test.live_correct_keystrokes += 1;
        } else {
            self.test.live_incorrect_keystrokes += 1;
        }

        if !is_keystroke_correct {
            self.test.total_errors_ever += 1;
        }

        if word_idx < self.test.word_stream.len() && c == ' ' {
            let user_current_word = current_input_segments.last().unwrap_or(&"").to_string();
            self.handle_space_press(word_idx, &user_current_word);
        }

        self.test.input.push(c);

        if c == ' ' {
            self.on_word_finished();
        }
        self.sync_display_text();
        self.check_scroll_trigger();
        self.check_test_completion();
    }

    pub fn on_backspace(&mut self) {
        if self.test.state == AppState::Finished { return; }

        if self.test.input.ends_with(' ') {
            let segments: Vec<&str> = self.test.input.split(' ').collect();
            if segments.len() >= 2 {
                let last_completed_idx = segments.len() - 2;
                let typed_word = segments[last_completed_idx];

                if let Some(target_word) = self.test.word_stream.get(last_completed_idx) {
                    if typed_word == target_word.text {
                        return;
                    }
                }

                let current_idx = last_completed_idx + 1;
                if current_idx < self.test.word_stream.len() {
                    self.test.word_stream[current_idx].state = WordState::Pending;
                }
                if last_completed_idx < self.test.word_stream.len() {
                    self.test.word_stream[last_completed_idx].state = WordState::Active;
                }
            }
        }

        if let Some(popped_char) = self.test.input.pop() {
            if popped_char == ' ' {
                // clear missed record so the word is treated as fresh when re-typed
                let word_idx = self.test.input.split(' ').count().saturating_sub(1);
                self.test.missed_chars.remove(&word_idx);
            }
            self.sync_display_text();
        }
    }

    pub(crate) fn words_visually_equal(typed: &str, target: &str) -> bool {
        let mut t = typed.chars();
        let mut g = target.chars();
        loop {
            let pair = (t.next(), g.next());
            if let (Some(a), Some(b)) = pair {
                if !strings::are_characters_visually_equal(a, b) { return false; }
            } else {
                return pair == (None, None);
            }
        }
    }

    pub(crate) fn handle_space_press(&mut self, word_idx: usize, user_current_word: &str) {
        let target_word = self.test.word_stream[word_idx].text.clone();

        // visual equality so "-" typed against "—" is not counted as an error
        let is_word_error = !Self::words_visually_equal(user_current_word, &target_word);
        // char counts; byte lengths are wrong for multi-byte chars like em dash (3 bytes, 1 char)
        let user_chars   = user_current_word.chars().count();
        let target_chars = target_word.chars().count();
        let extra_len_penalty = user_chars.saturating_sub(target_chars);

        if !self.test.processed_word_errors.contains(&word_idx) && (is_word_error || extra_len_penalty > 0) {
            let word_penalty = if is_word_error { 1 } else { 0 };
            self.test.total_errors_ever += word_penalty + extra_len_penalty;
            self.test.processed_word_errors.insert(word_idx);
        }

        if user_chars < target_chars {
            let missing_count = target_chars - user_chars;
            self.test.missed_chars.insert(word_idx, missing_count);
        }
    }
}
