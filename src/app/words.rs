use super::state::{App, CharSlot, SlotKind, SlotState};
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
        if let Some(word) = self.test.overflow_pool.pop() {
            let idx      = self.test.next_word_index;
            let new_word = crate::models::Word::new(word.clone(), idx);
            self.test.word_stream.push(new_word);
            self.test.next_word_index += 1;
            self.append_slots_for_words(&[word]);
            self.test.word_stream_string = self.test.word_stream
                .iter()
                .map(|w| w.text.as_str())
                .collect::<Vec<&str>>()
                .join(" ");
            return;
        }

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

            self.append_slots_for_words(&new_words.iter().map(|w| w.text.clone()).collect::<Vec<_>>());

            self.test.word_stream_string = self.test.word_stream
                .iter()
                .map(|w| w.text.as_str())
                .collect::<Vec<&str>>()
                .join(" ");
        }
    }

    pub(crate) fn append_slots_for_words(&mut self, words: &[String]) {
        let mut group_id = self.test.slots.last()
            .map(|s| s.group_id + 1)
            .unwrap_or(0);

        for (i, word) in words.iter().enumerate() {
            let needs_leading_space = !self.test.slots.is_empty() || i > 0;
            if needs_leading_space {
                self.test.slots.push(CharSlot {
                    expected: ' ',
                    kind: SlotKind::Space,
                    group_id,
                    visual_width: 1,
                    state: SlotState::Pending,
                });
                group_id += 1;
            }

            for ch in word.chars() {
                let (kind, visual_width) = match ch {
                    '\n' => (SlotKind::Newline, 1),
                    '\t' => (SlotKind::Tab,     2),
                    ' '  => (SlotKind::Space,   1),
                    _    => (SlotKind::Regular, 1),
                };
                self.test.slots.push(CharSlot {
                    expected: ch,
                    kind,
                    group_id,
                    visual_width,
                    state: SlotState::Pending,
                });
                if matches!(ch, ' ' | '\n') {
                    group_id += 1;
                }
            }
        }
    }

    pub(crate) fn update_stream_string(&mut self) {
        self.test.word_stream_string = self.test.word_stream
            .iter()
            .map(|w| w.text.as_str())
            .collect::<Vec<&str>>()
            .join(" ");

        self.rebuild_slots_from_stream();
    }

    pub(crate) fn rebuild_slots_from_stream(&mut self) {
        let text = &self.test.word_stream_string;
        let mut blank_lines: usize = 0;
        for line in text.split('\n') {
            if line.trim().is_empty() {
                blank_lines += 1;
            }
        }
        if text.ends_with('\n') {
            blank_lines = blank_lines.saturating_sub(1);
        }
        self.test.total_code_words = text.split_whitespace().count() + blank_lines;
        let mut slots: Vec<CharSlot> = Vec::with_capacity(text.len());
        let mut group_id = 0usize;

        for ch in text.chars() {
            let (kind, visual_width) = match ch {
                '\n' => (SlotKind::Newline, 1),
                '\t' => (SlotKind::Tab,     2),
                ' '  => (SlotKind::Space,   1),
                _    => (SlotKind::Regular, 1),
            };

            slots.push(CharSlot {
                expected: ch,
                kind,
                group_id,
                visual_width,
                state: SlotState::Pending,
            });

            if matches!(ch, ' ' | '\n') {
                group_id += 1;
            }
        }

        self.test.slots = slots;
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
