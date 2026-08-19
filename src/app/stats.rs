use super::state::App;
use crate::models::AppState;
use crate::utils::strings;

impl App {
    pub(crate) fn code_typed_word_count(&self) -> usize {
        let mut typed = self.test.code_words_scrolled
            + self.test.input.chars().filter(|c| *c == ' ' || *c == '\n').count();
        if self.test.state == AppState::Finished && typed < self.test.total_code_words {
            typed = self.test.total_code_words;
        }
        typed
    }

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

    pub fn calculate_custom_stats_for_slice(&self, input_chars: &[char], display_str: &str, mask: &[u8])
        -> (isize, isize, usize, usize, usize, usize)
    {
        let mut acc_correct_score: isize = 0;
        for &m in mask { if m == 0 { acc_correct_score += 1; } }
        let mut acc_incorrect_score: isize = 0;

        let mut raw_cor = 0;
        let mut raw_inc = 0;
        let mut raw_ext = 0;
        let mut raw_mis = 0;

        let mut dot_cor = 0usize;
        let mut dot_inc = 0usize;
        let mut dot_ext = 0usize;
        let mut dot_mis = 0usize;

        let display_chars: Vec<char> = display_str.chars().collect();

        fn is_visual_only(c: char) -> bool {
            c == '↵' || c == '·'
        }

        let is_code = self.config.word_data.name.starts_with("code_");

        let mut aligned_offsets: Vec<usize> = Vec::with_capacity(display_chars.len());
        let mut aligned_idx = 0usize;
        for (k, &dc) in display_chars.iter().enumerate() {
            aligned_offsets.push(aligned_idx);
            let is_extra = mask.get(k).copied().unwrap_or(0) == 2;
            if !is_visual_only(dc) || is_extra {
                aligned_idx += 1;
            }
        }

        let mut i = 0;
        while i < display_chars.len() {
            let mut word_end = i;
            while word_end < display_chars.len() {
                if display_chars[word_end] == ' '
                    || (is_code && display_chars[word_end] == '↵')
                {
                    break;
                }
                word_end += 1;
            }

            let mut word_has_error = false;

            for k in i..word_end {
                let target_char = display_chars[k];
                if is_visual_only(target_char) {
                    let m = mask.get(k).copied().unwrap_or(0);
                    if m != 0 && m != 4 { word_has_error = true; }
                    continue;
                }
                let is_extra   = if k < mask.len() { mask[k] == 2 } else { false };
                let ai         = aligned_offsets[k];
                let input_char = input_chars.get(ai).copied().unwrap_or('\0');

                if is_extra {
                    word_has_error = true;
                } else if input_char == '\0' {
                    word_has_error = true;
                } else if !strings::are_characters_visually_equal(input_char, target_char) {
                    word_has_error = true;
                }
            }

            for k in i..word_end {
                let target_char = display_chars[k];
                if is_visual_only(target_char) {
                    let m = mask.get(k).copied().unwrap_or(0);
                    let is_dot = target_char == '·';
                    match m {
                        2 => { acc_incorrect_score += 1; if is_dot { dot_ext += 1; } else { raw_ext += 1; } }
                        3 => { acc_correct_score -= 1;   if is_dot { dot_mis += 1; } else { raw_mis += 1; } }
                        4 => { if is_dot { dot_cor += 1; } else { raw_cor += 1; } }
                        1 => {
                            acc_correct_score -= 1;
                            acc_incorrect_score += 1;
                            if is_dot { dot_inc += 1; } else { raw_inc += 1; }
                        }
                        _ => { if !word_has_error { if is_dot { dot_cor += 1; } else { raw_cor += 1; } } }
                    }
                    continue;
                }
                let is_extra   = if k < mask.len() { mask[k] == 2 } else { false };
                let ai         = aligned_offsets[k];
                let input_char = input_chars.get(ai).copied().unwrap_or('\0');

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
                let boundary_char = display_chars[word_end];
                let boundary_mask = if word_end < mask.len() { mask[word_end] } else { 0 };
                if is_code {
                    match boundary_mask {
                        0 => {
                            let typed = match boundary_char {
                                '↵' => {
                                    let before = display_chars[..word_end]
                                        .iter().filter(|&&c| c == '↵').count();
                                    self.test.input.chars()
                                        .filter(|&c| c == '\n').count() > before
                                }
                                _ => {
                                    let before = display_chars[..word_end]
                                        .iter().filter(|&&c| c == ' ').count();
                                    self.test.input.chars()
                                        .filter(|&c| c == ' ').count() > before
                                }
                            };
                            if !typed {
                            } else if word_has_error {
                                acc_correct_score -= 1;
                                acc_incorrect_score += 1;
                            } else {
                                raw_cor += 1;
                            }
                        }
                        1 => {
                            acc_correct_score -= 1;
                            acc_incorrect_score += 1;
                            raw_inc += 1;
                        }
                        2 => {
                            acc_incorrect_score += 1;
                            raw_ext += 1;
                        }
                        4 => {
                            acc_correct_score += 1;
                            raw_cor += 1;
                        }
                        _ => {
                            acc_correct_score -= 1;
                            raw_mis += 1;
                        }
                    }
                } else {
                    let space_is_wrong = boundary_mask != 0;
                    if word_has_error || space_is_wrong {
                        acc_correct_score -= 1;
                        acc_incorrect_score += 1;
                    } else {
                        raw_cor += 1;
                    }
                }
                i = word_end + 1;
            } else {
                i = word_end;
            }
        }

        raw_cor += dot_cor / 2;
        raw_inc += dot_inc / 2;
        raw_ext += dot_ext / 2;
        raw_mis += dot_mis / 2;

        (acc_correct_score, acc_incorrect_score, raw_cor, raw_inc, raw_ext, raw_mis)
    }

    pub(crate) fn calculate_live_correct_chars(&self) -> usize {
        let is_code = self.config.word_data.name.starts_with("code_");
        if is_code {
            let (_, _, cor, _, _, _) = self.calculate_custom_stats_for_slice(
                &self.test.aligned_input,
                &self.test.display_string,
                &self.test.display_mask,
            );
            return cor;
        }

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
        let completed_mask: Vec<u8> = self.test.display_mask.iter().take(completed_len).copied().collect();

        let (_, _, completed_correct_chars, _, _, _) =
            self.calculate_custom_stats_for_slice(completed_aligned, &completed_display, &completed_mask);

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
