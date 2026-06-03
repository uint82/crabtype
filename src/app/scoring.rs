use super::state::App;
use crate::models::{AppState, Mode};
use crate::history;

impl App {
    pub fn check_time(&mut self) {
        if self.test.state != AppState::Running { return; }
        self.record_snapshot_if_needed();
        if let Some(start) = self.test.start_time {
            let elapsed = start.elapsed().as_secs_f64();
            if let Mode::Time(limit) = self.config.mode {
                if elapsed >= limit as f64 { self.end_test(); }
            }
        }
    }

    pub fn end_test(&mut self) {
        self.test.state = AppState::Finished;
        let duration_secs = self.test.start_time.map(|t| t.elapsed().as_secs_f64()).unwrap_or(1.0);

        if let Mode::Time(_) = self.config.mode {
            let typed_len = self.test.aligned_input.len();
            if typed_len < self.test.display_string.chars().count() {
                let truncated: String = self.test.display_string.chars().take(typed_len).collect();
                self.test.display_string = truncated;
                self.test.display_mask.truncate(typed_len);
            }
        }

        let total_correct_chars = self.test.st_correct + self.calculate_live_correct_chars();

        self.test.final_raw_wpm = (self.test.gross_char_count as f64 / 5.0) * (60.0 / duration_secs);
        self.test.final_wpm    = (total_correct_chars as f64 / 5.0) * (60.0 / duration_secs);

        let total_keystrokes = self.test.live_correct_keystrokes + self.test.live_incorrect_keystrokes;
        self.test.final_accuracy = if total_keystrokes > 0 {
            (self.test.live_correct_keystrokes as f64 / total_keystrokes as f64) * 100.0
        } else {
            0.0
        };

        self.test.final_time = duration_secs;
        self.show_ui = true;

        let last_full_second = if self.test.last_snapshot_second == u64::MAX {
            0.0
        } else {
            self.test.last_snapshot_second as f64
        };
        let remaining = duration_secs - last_full_second;

        if remaining >= 0.495 {
            self.push_snapshot(duration_secs);
        }

        self.test.final_consistency = self.calculate_consistency();

        self.check_personal_best();

        if let Some(ref mut d) = self.discord {
            let typed_words = self.test.scrolled_word_count
                + self.test.input.split_whitespace().count();
            let total_words = match self.config.mode {
                Mode::Words(w) => w,
                _ => self.test.total_quote_words.max(self.test.word_stream.len()),
            };
            use crate::ui::utils::get_quote_length_category;
            let ql = get_quote_length_category(self.test.original_quote_length);
            d.set_result(
                self.test.final_wpm,
                self.test.final_accuracy,
                &self.config.mode,
                self.test.is_new_best,
                typed_words,
                total_words,
                &self.test.current_quote_source.clone(),
                self.test.final_consistency,
                self.config.use_punctuation,
                self.config.use_numbers,
                ql,
                &self.config.word_data.name,
            );
        }

        if !self.test.cumulative_words.is_empty() {
            self.last_test_words = Some(self.test.cumulative_words.clone());
        }
        let _ = history::record_test(self, true);
    }

    pub fn record_snapshot_if_needed(&mut self) {
        if self.test.state != AppState::Running { return; }
        if let Some(start) = self.test.start_time {
            let elapsed_secs = start.elapsed().as_secs_f64();
            let current_second = elapsed_secs.floor() as u64;

            if current_second >= 1 &&
               (self.test.last_snapshot_second == u64::MAX || current_second > self.test.last_snapshot_second)
            {
                self.test.last_snapshot_second = current_second;
                self.push_snapshot(current_second as f64);
            }
        }
    }

    pub(crate) fn push_snapshot(&mut self, elapsed_secs: f64) {
        if elapsed_secs <= 0.0 { return; }

        let total_correct_chars = self.test.st_correct + self.calculate_live_correct_chars();
        let raw_wpm = (self.test.gross_char_count as f64 / 5.0) * (60.0 / elapsed_secs);
        let net_wpm = (total_correct_chars as f64 / 5.0) * (60.0 / elapsed_secs);

        let errors_this_second = self.test.live_incorrect_keystrokes
            .saturating_sub(self.test.prev_incorrect_keystrokes) as f64;
        self.test.prev_incorrect_keystrokes = self.test.live_incorrect_keystrokes;

        let burst_chars = self.test.gross_char_count
            .saturating_sub(self.test.prev_gross_char_count);
        self.test.prev_gross_char_count = self.test.gross_char_count;
        let burst_wpm = (burst_chars as f64 / 5.0) * 60.0;
        self.test.burst_wpm_history.push(burst_wpm);

        self.test.wpm_history.push((elapsed_secs, net_wpm));
        self.test.raw_wpm_history.push((elapsed_secs, raw_wpm));
        self.test.errors_history.push((elapsed_secs, errors_this_second));
    }

    pub(crate) fn check_test_completion(&mut self) {
        match self.config.mode {
            Mode::Words(_) | Mode::Quote(_) => {
                // subtract extras only. aligned_input includes \0 slots for missed chars
                let effective_len = self.test.aligned_input.len()
                    .saturating_sub(self.test.extra_char_count);
                if effective_len < self.test.word_stream_string.chars().count() { return; }

                let target_words: Vec<&str> = self.test.word_stream_string.split(' ').collect();
                let input_words:  Vec<&str> = self.test.input.split(' ').collect();

                if let Some(last_target_word) = target_words.last() {
                    let last_word_index = target_words.len() - 1;
                    let last_input_word = input_words.get(last_word_index).unwrap_or(&"");
                    if last_input_word == last_target_word {
                        self.end_test();
                    }
                }
            }
            _ => {}
        }
    }

    pub(crate) fn check_personal_best(&mut self) {
        let (mode_str, mode_value) = match &self.config.mode {
            Mode::Time(t)  => ("time".to_string(),  t.to_string()),
            Mode::Words(w) => ("words".to_string(), w.to_string()),
            Mode::Quote(q) => {
                use crate::models::QuoteSelector;
                use crate::ui::utils::get_quote_length_category;
                let label = match q {
                    QuoteSelector::Id(_) => get_quote_length_category(self.test.original_quote_length).to_string(),
                    QuoteSelector::Category(len) => {
                        let s = format!("{:?}", len).to_lowercase();
                        if s == "all" {
                            get_quote_length_category(self.test.original_quote_length).to_string()
                        } else {
                            s
                        }
                    }
                };
                ("quote".to_string(), label)
            }
        };

        if let Ok(records) = history::load_history() {
            let prev_best = records.iter()
                .filter(|r| r.completed && r.mode == mode_str && r.mode_value == mode_value)
                .filter_map(|r| r.wpm)
                .fold(0.0_f64, f64::max);
            self.test.is_new_best = self.test.final_wpm > prev_best;
        }
    }

    pub(crate) fn calculate_consistency(&self) -> f64 {
        let wpms: Vec<f64> = self.test.burst_wpm_history.iter()
            .copied()
            .filter(|&w| w > 0.0)
            .collect();
        let n = wpms.len();
        if n < 2 { return 100.0; }
        let mean = wpms.iter().sum::<f64>() / n as f64;
        if mean == 0.0 { return 100.0; }
        let variance = wpms.iter().map(|w| (w - mean).powi(2)).sum::<f64>() / n as f64;
        let std_dev = variance.sqrt();
        let cv = std_dev / mean;
        (1.0 - cv).clamp(0.0, 1.0) * 100.0
    }
}
