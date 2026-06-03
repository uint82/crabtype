use super::state::App;
use crate::models::{Mode, WordState};

impl App {
    pub(crate) fn generate_initial_words(&mut self) {
        let result = self.config.word_generator.generate_initial_words(
            &self.config.mode,
            self.config.quote_data.as_ref(),
        );
        self.test.word_stream          = result.word_stream;
        self.test.quote_pool           = result.quote_pool;
        self.test.total_quote_words    = result.total_quote_words;
        self.test.current_quote_source = result.current_quote_source;
        self.test.generated_count      = result.generated_count;
        self.test.next_word_index      = result.next_index;
        self.test.cumulative_words     = self.test.word_stream.iter().map(|w| w.text.clone()).collect();
        self.update_stream_string();
        self.sync_display_text();

        if matches!(self.config.mode, Mode::Quote(_)) {
            self.test.original_quote_length = self.test.word_stream_string.chars().count();
        }
    }

    pub(crate) fn add_one_word(&mut self) {
        if let Some((new_words, new_next_index)) = self.config.word_generator.add_one_word(
            &self.config.mode,
            &self.test.word_stream,
            &mut self.test.quote_pool,
            self.test.generated_count,
            self.test.next_word_index,
        ) {
            self.test.word_stream.extend(new_words.iter().cloned());
            self.test.cumulative_words.extend(new_words.iter().map(|w| w.text.clone()));
            self.test.next_word_index = new_next_index;
            if matches!(self.config.mode, Mode::Words(_)) {
                self.test.generated_count += new_words.len();
            }
            self.update_stream_string();
        }
    }

    pub(crate) fn update_stream_string(&mut self) {
        self.test.word_stream_string = self.test.word_stream
            .iter()
            .map(|w| w.text.as_str())
            .collect::<Vec<&str>>()
            .join(" ");
    }

    pub(crate) fn on_word_finished(&mut self) {
        let segments: Vec<&str> = self.test.input.split(' ').collect();
        let finished_idx = segments.len().saturating_sub(2);
        if finished_idx < self.test.word_stream.len() {
            self.test.word_stream[finished_idx].state = WordState::Typed;
        }
        let next_idx = finished_idx + 1;
        if next_idx < self.test.word_stream.len() {
            self.test.word_stream[next_idx].state = WordState::Active;
        }
        if finished_idx >= self.test.furthest_word_idx {
            self.test.furthest_word_idx = finished_idx + 1;
            let pending_count = self.test.word_stream.iter()
                .skip(next_idx)
                .filter(|w| w.state == WordState::Pending)
                .count();
            if pending_count < 100 {
                self.add_one_word();
            }
        }
    }
}
