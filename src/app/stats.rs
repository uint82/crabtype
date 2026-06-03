use super::state::App;
use crate::utils::strings;

impl App {
    pub fn resolved_char_stats(&self) -> (usize, usize, usize, usize) {
        let (_, _, vis_cor, vis_inc, vis_ext, vis_mis) =
            self.calculate_custom_stats_for_slice(
                &self.test.aligned_input,
                &self.test.display_string,
                &self.test.display_mask,
            );
        (
            self.test.st_correct   + vis_cor,
            self.test.st_incorrect + vis_inc,
            self.test.st_extra     + vis_ext,
            self.test.st_missed    + vis_mis,
        )
    }

    pub fn calculate_custom_stats_for_slice(&self, input_chars: &[char], display_str: &str, mask: &[bool])
        -> (isize, isize, usize, usize, usize, usize)
    {
        let mut acc_correct_score: isize = 0;
        for &m in mask { if !m { acc_correct_score += 1; } }
        let mut acc_incorrect_score: isize = 0;

        let mut raw_cor = 0;
        let mut raw_inc = 0;
        let mut raw_ext = 0;
        let mut raw_mis = 0;

        let display_chars: Vec<char> = display_str.chars().collect();

        let mut i = 0;
        while i < display_chars.len() {
            let mut word_end = i;
            while word_end < display_chars.len() {
                let is_extra = if word_end < mask.len() { mask[word_end] } else { false };
                if !is_extra && display_chars[word_end] == ' ' { break; }
                word_end += 1;
            }

            let mut word_has_error = false;

            for k in i..word_end {
                let is_extra   = if k < mask.len() { mask[k] } else { false };
                let target_char = display_chars[k];
                let input_char  = input_chars.get(k).copied().unwrap_or('\0');

                if is_extra {
                    word_has_error = true;
                } else if input_char == '\0' {
                    word_has_error = true;
                } else if !strings::are_characters_visually_equal(input_char, target_char) {
                    word_has_error = true;
                }
            }

            for k in i..word_end {
                let is_extra   = if k < mask.len() { mask[k] } else { false };
                let target_char = display_chars[k];
                let input_char  = input_chars.get(k).copied().unwrap_or('\0');

                if is_extra {
                    acc_incorrect_score += 1;
                    raw_ext += 1;
                } else if input_char == '\0' {
                    acc_correct_score -= 1;
                    raw_mis += 1;
                } else if !strings::are_characters_visually_equal(input_char, target_char) {
                    acc_correct_score -= 1;
                    acc_incorrect_score += 1;
                    raw_inc += 1;
                } else if !word_has_error {
                    raw_cor += 1;
                }
            }

            if word_end < display_chars.len() {
                if word_has_error {
                    acc_correct_score -= 1;
                    acc_incorrect_score += 1;
                } else {
                    raw_cor += 1;
                }
                i = word_end + 1;
            } else {
                i = word_end;
            }
        }

        (acc_correct_score, acc_incorrect_score, raw_cor, raw_inc, raw_ext, raw_mis)
    }

    pub(crate) fn calculate_live_correct_chars(&self) -> usize {
        let ends_with_space = self.test.aligned_input.last() == Some(&' ');

        let completed_len = if ends_with_space || self.test.aligned_input.is_empty() {
            self.test.aligned_input.len()
        } else {
            self.test.aligned_input.iter().rposition(|&c| c == ' ')
                .map(|p| p + 1)
                .unwrap_or(0)
        };

        let completed_aligned = &self.test.aligned_input[..completed_len];
        let completed_display: String = self.test.display_string.chars().take(completed_len).collect();
        let completed_mask: Vec<bool> = self.test.display_mask.iter().take(completed_len).copied().collect();

        let (_, _, completed_correct_chars, _, _, _) =
            self.calculate_custom_stats_for_slice(completed_aligned, &completed_display, &completed_mask);

        // use self.input for the in-progress word. it has no \0 so indexing is unambiguous
        let current_word_input = if ends_with_space || self.test.aligned_input.is_empty() {
            ""
        } else if let Some(last_space) = self.test.input.rfind(' ') {
            &self.test.input[last_space + 1..]
        } else {
            self.test.input.as_str()
        };

        let current_word_correct_chars = if !current_word_input.is_empty() {
            let current_word_idx = self.test.input.split(' ').count().saturating_sub(1);
            if let Some(word) = self.test.word_stream.get(current_word_idx) {
                let target_word = &word.text;
                let has_error = current_word_input.chars().enumerate().any(|(i, c)| {
                    target_word.chars().nth(i).map_or(true, |tc| !strings::are_characters_visually_equal(c, tc))
                });
                if has_error { 0 } else { current_word_input.chars().count() }
            } else {
                0
            }
        } else {
            0
        };

        completed_correct_chars + current_word_correct_chars
    }
}
