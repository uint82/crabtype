use super::state::App;

impl App {
    pub(crate) fn check_scroll_trigger(&mut self) {
        let mut running_char_count = 0;
        let mut current_line_index = 0;
        for (i, line) in self.test.visual_lines.iter().enumerate() {
            let line_len = line.chars().count() + 1;
            if self.test.aligned_input.len() < running_char_count + line_len {
                current_line_index = i;
                break;
            }
            running_char_count += line_len;
        }
        if current_line_index >= 2 {
            self.delete_first_visual_line();
        }
    }

    pub(crate) fn delete_first_visual_line(&mut self) {
        if self.test.visual_lines.is_empty() { return; }
        let first_line = &self.test.visual_lines[0];
        let visual_char_count = first_line.chars().count();
        let mut chars_to_remove_visual = visual_char_count;

        if self.test.display_string.chars().count() > visual_char_count {
            if let Some(c) = self.test.display_string.chars().nth(visual_char_count) {
                if c == ' ' { chars_to_remove_visual += 1; }
            }
        }

        // aligned_input has \0 for missed positions, so stats are accurate even for short words
        let capped = chars_to_remove_visual.min(self.test.aligned_input.len());
        let aligned_chunk  = &self.test.aligned_input[..capped];
        let display_chunk: String    = self.test.display_string.chars().take(chars_to_remove_visual).collect();
        let mask_chunk: Vec<bool>    = self.test.display_mask.iter().take(chars_to_remove_visual).cloned().collect();

        let (acc_cor, acc_inc, raw_cor, raw_inc, raw_ext, raw_mis) =
            self.calculate_custom_stats_for_slice(aligned_chunk, &display_chunk, &mask_chunk);

        self.test.st_correct   += raw_cor;
        self.test.st_incorrect += raw_inc;
        self.test.st_extra     += raw_ext;
        self.test.st_missed    += raw_mis;

        self.test.acc_score_correct   = (self.test.acc_score_correct   + acc_cor).max(0);
        self.test.acc_score_incorrect = (self.test.acc_score_incorrect + acc_inc).max(0);

        self.test.uncorrected_errors_scrolled += raw_inc + raw_mis + raw_ext;

        let tokens_scrolled = aligned_chunk.iter().filter(|&&c| c == ' ').count();
        if tokens_scrolled > 0 {
            self.test.scrolled_word_count += tokens_scrolled;
            let drain_amount = tokens_scrolled.min(self.test.word_stream.len());
            self.test.word_stream.drain(0..drain_amount);
            self.test.furthest_word_idx = self.test.furthest_word_idx.saturating_sub(tokens_scrolled);

            // word indices shift down after scrolling, so remap both maps to stay in sync
            self.test.missed_chars = self.test.missed_chars
                .iter()
                .filter(|(&k, _)| k >= tokens_scrolled)
                .map(|(&k, &v)| (k - tokens_scrolled, v))
                .collect();

            self.test.processed_word_errors = self.test.processed_word_errors
                .iter()
                .filter(|&&k| k >= tokens_scrolled)
                .map(|&k| k - tokens_scrolled)
                .collect();
        }

        let mut real_chars_removed = 0;
        for i in 0..chars_to_remove_visual {
            if i < self.test.display_mask.len() {
                if !self.test.display_mask[i] { real_chars_removed += 1; }
            }
        }
        if real_chars_removed > 0 {
            // real_chars_removed is a char count. must convert to byte offset before slicing
            let ws_byte_len: usize = self.test.word_stream_string.chars()
                .take(real_chars_removed)
                .map(|c| c.len_utf8())
                .sum();
            if self.test.word_stream_string.len() >= ws_byte_len {
                self.test.word_stream_string = self.test.word_stream_string[ws_byte_len..].to_string();
            }
        }

        // self.input has no \0 so we count real chars from aligned_chunk to know how many to drain
        let clean_chars_to_remove = aligned_chunk.iter().filter(|&&c| c != '\0').count();
        let byte_len: usize = self.test.input.chars()
            .take(clean_chars_to_remove)
            .map(|c| c.len_utf8())
            .sum();
        self.test.input.drain(..byte_len);

        self.sync_display_text();
    }
}
