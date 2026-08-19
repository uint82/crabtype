use super::state::{App, CharSlot, SlotKind, SlotState};
use crate::models::{AppState, Mode, WordState};
use crate::utils::strings;
use std::time::Instant;

impl App {
    pub fn on_key(&mut self, c: char) {
        if self.test.state == AppState::Waiting && c == ' ' { return; }
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

        let is_code = self.config.word_data.name.starts_with("code_");

        if is_code {
            self.on_key_code(c);
            return;
        }

        let current_input_segments: Vec<&str> = self.test.input.split(' ').collect();
        let word_idx = current_input_segments.len().saturating_sub(1);

        let mut is_extra_char = false;

        if word_idx < self.test.word_stream.len() {
            let target_word_struct = &self.test.word_stream[word_idx];
            let target_word = &target_word_struct.text;
            let user_current_word = current_input_segments.last().unwrap_or(&"");

            if c == ' ' && user_current_word.is_empty() { return; }

            let target_char_count = target_word.chars().count();
            let user_char_count = user_current_word.chars().count();

            let limit = target_char_count + 19;
            if user_char_count >= limit {
                if c != ' ' { return; }
            }

            if c != ' ' {
                let is_extra = user_char_count >= target_char_count;
                is_extra_char = is_extra;
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

        let is_keystroke_correct = if word_idx < self.test.word_stream.len() {
            let target_word = &self.test.word_stream[word_idx].text;
            let user_current_word = current_input_segments.last().unwrap_or(&"");

            if c == ' ' {
                Self::words_visually_equal(user_current_word, target_word)
            } else {
                let user_char_count = user_current_word.chars().count();
                let target_char_count = target_word.chars().count();
                if user_char_count < target_char_count {
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

        let cursor = self.test.cursor_idx;

        if c == ' ' {
            let boundary_pos = self.test.slots[cursor..]
                .iter()
                .position(|s| matches!(s.kind, SlotKind::Space | SlotKind::Newline))
                .map(|rel| cursor + rel)
                .unwrap_or(cursor);

            let boundary_state = if is_keystroke_correct {
                SlotState::Correct
            } else {
                SlotState::Wrong(c)
            };
            if boundary_pos < self.test.slots.len() {
                self.test.slots[boundary_pos].state = boundary_state;
            }

            let group = self.test.slots.get(boundary_pos).map(|s| s.group_id).unwrap_or(0);
            for slot in self.test.slots[..boundary_pos].iter_mut().rev() {
                if slot.group_id != group { break; }
                if slot.state == SlotState::Pending {
                    slot.state = SlotState::Missed;
                }
            }
        } else {
            if is_extra_char {
                let group_id = self.test.slots.get(cursor)
                    .map(|s| s.group_id)
                    .or_else(|| self.test.slots.last().map(|s| s.group_id))
                    .unwrap_or(0);
                self.test.slots.insert(cursor, CharSlot {
                    expected: c,
                    kind: SlotKind::Regular,
                    group_id,
                    visual_width: 1,
                    state: SlotState::Extra(c),
                });
            } else {
                let slot_state = if is_keystroke_correct {
                    SlotState::Correct
                } else {
                    SlotState::Wrong(c)
                };
                if cursor < self.test.slots.len() {
                    self.test.slots[cursor].state = slot_state;
                }
            }
        }

        self.test.input.push(c);

        if c == ' ' {
            self.on_word_finished();
        }
        self.sync_display_text();
        self.check_scroll_trigger();
        self.check_test_completion();
    }

    fn on_key_code(&mut self, c: char) {
        let cursor = match self.test.slots.iter().position(|s| s.state == SlotState::Pending) {
            Some(i) => i,
            None    => return,
        };

        if let Some(glyph) = self.uncommitted_glyph() {
            if c == ' ' {
                self.insert_glyph_extra_after(glyph, ' ');
                self.commit_uncommitted(glyph);
            } else {
                self.insert_extra_after(glyph, c);
            }
            return;
        }

        if cursor >= self.test.slots.len() { return; }

        let slot_kind  = self.test.slots[cursor].kind.clone();
        let slot_expected = self.test.slots[cursor].expected;

        match slot_kind {
            SlotKind::Newline | SlotKind::Tab => {
                if c == ' ' && slot_kind == SlotKind::Newline {
                    let group = self.test.slots[cursor].group_id;
                    let line_has_error = self.test.slots[..=cursor]
                        .iter()
                        .filter(|s| s.group_id == group)
                        .any(|s| matches!(s.state,
                            SlotState::Wrong(_) | SlotState::Uncommitted(_)
                                | SlotState::Missed | SlotState::Extra(_)
                        ));
                    self.show_ui = false;
                    self.test.gross_char_count += 1;
                    self.test.slots[cursor].state = SlotState::Missed;
                    if !line_has_error {
                        self.test.live_correct_keystrokes += 1;
                    } else {
                        self.test.live_incorrect_keystrokes += 1;
                        self.test.total_errors_ever += 1;
                    }
                    self.test.input.push('\n');
                    self.sync_display_text();
                    self.check_scroll_trigger();
                    self.check_test_completion();
                    return;
                }

                if c == ' ' && slot_kind == SlotKind::Tab {
                    let group = self.test.slots[cursor].group_id;
                    let any_typed = self.test.slots[..cursor]
                        .iter()
                        .rev()
                        .take_while(|s| s.group_id == group)
                        .any(|s| matches!(s.state,
                            SlotState::Correct | SlotState::Wrong(_) | SlotState::Extra(_)
                        ));
                    if !any_typed { return; }
                }

                self.show_ui = false;
                self.test.gross_char_count += 1;
                self.test.live_incorrect_keystrokes += 1;
                self.test.total_errors_ever += 1;
                if slot_kind == SlotKind::Tab {
                    self.test.slots[cursor].state = SlotState::Wrong(c);
                    for _ in 0..self.test.slots[cursor].visual_width {
                        self.test.input.push('\t');
                    }
                } else {
                    self.test.slots[cursor].state = SlotState::Uncommitted(c);
                    self.test.input.push(c);
                }
                self.sync_display_text();
                self.check_scroll_trigger();
                self.check_test_completion();
                return;
            }
            SlotKind::Space => {
                if c == ' ' {
                    let group = self.test.slots[cursor].group_id;
                    let any_typed = self.test.slots[..cursor]
                        .iter()
                        .rev()
                        .take_while(|s| s.group_id == group)
                        .any(|s| matches!(s.state,
                            SlotState::Correct | SlotState::Wrong(_) | SlotState::Extra(_)
                        ));
                    if !any_typed { return; }
                } else {
                    self.insert_extra_code(cursor, c);
                    return;
                }
            }
            SlotKind::Regular => {
                if c == ' ' {
                    let group = self.test.slots[cursor].group_id;
                    let any_typed = self.test.slots[..cursor]
                        .iter()
                        .rev()
                        .take_while(|s| s.group_id == group)
                        .any(|s| matches!(s.state,
                            SlotState::Correct | SlotState::Wrong(_) | SlotState::Extra(_)
                        ));
                    if !any_typed { return; }

                    let boundary_pos = match self.test.slots[cursor..]
                        .iter()
                        .position(|s| matches!(s.kind, SlotKind::Space | SlotKind::Newline))
                        .map(|rel| cursor + rel)
                    {
                        Some(p) => p,
                        None    => return,
                    };

                    self.show_ui = false;
                    self.test.gross_char_count += 1;

                    for slot in self.test.slots[cursor..boundary_pos].iter_mut() {
                        if slot.state == SlotState::Pending {
                            slot.state = SlotState::Missed;
                        }
                    }

                    let group = self.test.slots[boundary_pos].group_id;
                    let word_has_error = self.test.slots[..=boundary_pos]
                        .iter()
                        .filter(|s| s.group_id == group)
                        .any(|s| matches!(s.state,
                            SlotState::Wrong(_) | SlotState::Uncommitted(_)
                                | SlotState::Missed | SlotState::Extra(_)
                        ));

                    if self.test.slots[boundary_pos].kind == SlotKind::Newline {
                        self.test.slots[boundary_pos].state = SlotState::Missed;
                        if !word_has_error {
                            self.test.live_correct_keystrokes += 1;
                        } else {
                            self.test.live_incorrect_keystrokes += 1;
                            self.test.total_errors_ever += 1;
                        }
                        self.test.input.push('\n');
                    } else {
                        let boundary_state = if word_has_error {
                            SlotState::Wrong(c)
                        } else {
                            SlotState::Correct
                        };
                        self.test.slots[boundary_pos].state = boundary_state;

                        if word_has_error {
                            self.test.live_incorrect_keystrokes += 1;
                            self.test.total_errors_ever += 1;
                        } else {
                            self.test.live_correct_keystrokes += 1;
                        }
                        self.test.input.push(' ');
                    }

                    self.sync_display_text();
                    self.check_scroll_trigger();
                    self.check_test_completion();
                    return;
                }
            }
        }

        self.show_ui = false;
        self.test.gross_char_count += 1;

        let is_correct = strings::are_characters_visually_equal(c, slot_expected);

        if is_correct {
            self.test.live_correct_keystrokes += 1;
        } else {
            self.test.live_incorrect_keystrokes += 1;
            self.test.total_errors_ever += 1;
        }

        if slot_kind == SlotKind::Space {
            let boundary_state = if is_correct {
                SlotState::Correct
            } else {
                SlotState::Wrong(c)
            };
            self.test.slots[cursor].state = boundary_state;

            let group = self.test.slots[cursor].group_id;
            for slot in self.test.slots[..cursor].iter_mut().rev() {
                if slot.group_id != group { break; }
                if slot.state == SlotState::Pending {
                    slot.state = SlotState::Missed;
                }
            }
        } else {
            let slot_state = if is_correct {
                SlotState::Correct
            } else {
                SlotState::Wrong(c)
            };
            if cursor < self.test.slots.len() {
                self.test.slots[cursor].state = slot_state;
            }
        }

        self.test.input.push(c);

        self.sync_display_text();
        self.check_scroll_trigger();
        self.check_test_completion();
    }

    pub(crate) fn uncommitted_glyph(&self) -> Option<usize> {
        self.test.slots
            .iter()
            .position(|s| matches!(s.state, SlotState::Uncommitted(_)))
    }

    fn commit_uncommitted(&mut self, glyph: usize) {
        self.show_ui = false;
        self.test.gross_char_count += 1;
        self.test.live_incorrect_keystrokes += 1;
        self.test.total_errors_ever += 1;

        let typed = match self.test.slots[glyph].state {
            SlotState::Uncommitted(c) => c,
            _ => '\0',
        };
        self.test.slots[glyph].state = SlotState::Wrong(typed);

        match self.test.slots[glyph].kind {
            SlotKind::Newline => self.test.input.push('\n'),
            SlotKind::Tab => {
                for _ in 0..self.test.slots[glyph].visual_width {
                    self.test.input.push('\t');
                }
            }
            _ => {}
        }

        self.sync_display_text();
        self.check_scroll_trigger();
        self.check_test_completion();
    }

    fn insert_extra_after(&mut self, glyph: usize, c: char) {
        let group_id = self.test.slots[glyph].group_id;
        let mut extra_count = 0usize;
        for slot in &self.test.slots[glyph + 1..] {
            if matches!(slot.state, SlotState::Extra(_)) {
                extra_count += 1;
            } else {
                break;
            }
        }
        if extra_count >= 20 { return; }

        self.show_ui = false;
        self.test.gross_char_count += 1;
        self.test.live_incorrect_keystrokes += 1;
        self.test.total_errors_ever += 1;

        self.test.slots.insert(glyph + 1 + extra_count, CharSlot {
            expected: c,
            kind: SlotKind::Regular,
            group_id,
            visual_width: 1,
            state: SlotState::Extra(c),
        });

        self.test.input.push(c);
        self.sync_display_text();
        self.check_scroll_trigger();
        self.check_test_completion();
    }

    fn insert_glyph_extra_after(&mut self, glyph: usize, c: char) {
        let group_id = self.test.slots[glyph].group_id;
        let mut extra_count = 0usize;
        for slot in &self.test.slots[glyph + 1..] {
            if matches!(slot.state, SlotState::Extra(_)) {
                extra_count += 1;
            } else {
                break;
            }
        }
        self.test.slots.insert(glyph + 1 + extra_count, CharSlot {
            expected: c,
            kind: SlotKind::Regular,
            group_id,
            visual_width: 1,
            state: SlotState::Extra(c),
        });
    }

    fn insert_extra_code(&mut self, cursor: usize, c: char) {
        let group_id = self.test.slots[cursor].group_id;
        let extra_count = self.test.slots
            .iter()
            .filter(|s| s.group_id == group_id && matches!(s.state, SlotState::Extra(_)))
            .count();
        if extra_count >= 20 { return; }

        self.show_ui = false;
        self.test.gross_char_count += 1;
        self.test.live_incorrect_keystrokes += 1;
        self.test.total_errors_ever += 1;

        self.test.slots.insert(cursor, CharSlot {
            expected: c,
            kind: SlotKind::Regular,
            group_id,
            visual_width: 1,
            state: SlotState::Extra(c),
        });

        self.test.input.push(c);
        self.sync_display_text();
        self.check_scroll_trigger();
        self.check_test_completion();
    }

    pub fn on_backspace(&mut self) {
        if self.test.state == AppState::Finished { return; }

        let is_code = self.config.word_data.name.starts_with("code_");

        if !is_code {
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
        }

        let slot_pos_opt: Option<usize> = if is_code {
            self.test.slots.iter().position(|s| s.state == SlotState::Pending)
                .and_then(|i| if i > 0 { Some(i - 1) } else { None })
        } else {
            let cursor = self.test.cursor_idx;
            if cursor > 0 && cursor <= self.test.slots.len() {
                Some(cursor - 1)
            } else {
                None
            }
        };

       if is_code {
            if let Some(slot_pos) = slot_pos_opt {
                let block = match (&self.test.slots[slot_pos].kind, &self.test.slots[slot_pos].state) {
                    (SlotKind::Newline, SlotState::Correct) => true,
                    (SlotKind::Space, SlotState::Correct) => {
                        let group = self.test.slots[slot_pos].group_id;
                        !self.test.slots[..=slot_pos].iter().any(|s| {
                            s.group_id == group && matches!(s.state,
                                SlotState::Wrong(_) | SlotState::Uncommitted(_)
                                    | SlotState::Missed | SlotState::Extra(_)
                            )
                        })
                    }
                    _ => false,
                };
                if block {
                    return;
                }
            }
        }

        if let Some(popped_char) = self.test.input.pop() {
            if !is_code {
                if popped_char == ' ' {
                    let word_idx = self.test.input.split(' ').count().saturating_sub(1);
                    self.test.missed_chars.remove(&word_idx);
                }
            }

            if let Some(slot_pos) = slot_pos_opt {
                let group = self.test.slots[slot_pos].group_id;

                if matches!(self.test.slots[slot_pos].state, SlotState::Extra(_)) {
                    self.test.slots.remove(slot_pos);

                    if is_code {
                        let mut glyph = slot_pos.saturating_sub(1);
                        while glyph > 0
                            && matches!(self.test.slots[glyph].state, SlotState::Extra(_))
                        {
                            glyph -= 1;
                        }
                        let slot = &mut self.test.slots[glyph];
                        if slot.kind == SlotKind::Newline {
                            if let SlotState::Wrong(typed) = slot.state.clone() {
                                slot.state = SlotState::Uncommitted(typed);
                            }
                        }
                    }
                } else {
                    self.test.slots[slot_pos].state = SlotState::Pending;

                    if slot_pos > 0
                        && self.test.slots[slot_pos].kind == SlotKind::Space
                        && matches!(self.test.slots[slot_pos - 1].state, SlotState::Extra('\n'))
                    {
                        self.test.slots.remove(slot_pos - 1);
                    }

                    if popped_char == ' ' || (is_code && popped_char == '\n') {
                        for slot in self.test.slots[..slot_pos].iter_mut().rev() {
                            if slot.group_id != group { break; }
                            if slot.state == SlotState::Missed {
                                slot.state = SlotState::Pending;
                            }
                        }
                    }

                    if is_code
                        && slot_pos > 0
                        && self.test.slots[slot_pos].kind == SlotKind::Newline
                    {
                        let group = self.test.slots[slot_pos - 1].group_id;
                        for slot in self.test.slots[..slot_pos].iter_mut().rev() {
                            if slot.group_id != group { break; }
                            match slot.state {
                                SlotState::Missed => {
                                    slot.state = SlotState::Pending;
                                }
                                SlotState::Wrong('\n')
                                    if matches!(slot.kind, SlotKind::Regular | SlotKind::Tab) =>
                                {
                                    slot.state = SlotState::Pending;
                                }
                                _ => {}
                            }
                        }
                        if self.test.input.ends_with('\n') {
                            self.test.input.pop();
                        }
                    }
                }
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

        let is_word_error = !Self::words_visually_equal(user_current_word, &target_word);
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


    pub fn on_enter(&mut self) {
        if self.test.state == AppState::Finished { return; }
        if self.test.state == AppState::Waiting {
            self.test.start_time = Some(std::time::Instant::now());
            self.test.state = AppState::Running;
        }

        let cursor = match self.test.slots.iter().position(|s| s.state == SlotState::Pending) {
            Some(i) => i,
            None    => return,
        };

        if let Some(glyph) = self.uncommitted_glyph() {
            self.insert_glyph_extra_after(glyph, '\n');
            self.commit_uncommitted(glyph);
            return;
        }

        self.show_ui = false;
        self.test.gross_char_count += 1;

        let boundary_pos = self.test.slots[cursor..]
            .iter()
            .position(|s| matches!(s.kind, SlotKind::Space | SlotKind::Newline))
            .map(|rel| cursor + rel);

        if boundary_pos.is_none() {
            self.test.slots[cursor].state = SlotState::Wrong('\n');
            self.test.live_incorrect_keystrokes += 1;
            self.test.total_errors_ever += 1;
            self.test.input.push('\n');
            for slot in self.test.slots[cursor + 1..].iter_mut() {
                if slot.state == SlotState::Pending {
                    slot.state = SlotState::Missed;
                }
            }
            self.sync_display_text();
            self.check_test_completion();
            return;
        }

        let boundary_pos = boundary_pos.unwrap();
        let boundary_kind = self.test.slots[boundary_pos].kind.clone();

        if boundary_pos > cursor {
            self.test.slots[cursor].state = SlotState::Wrong('\n');
            for slot in self.test.slots[cursor + 1..boundary_pos].iter_mut() {
                if slot.state == SlotState::Pending {
                    slot.state = SlotState::Missed;
                }
            }
        }

        match boundary_kind {
            SlotKind::Space => {
                let group = self.test.slots[boundary_pos].group_id;
                for slot in self.test.slots[..boundary_pos].iter_mut().rev() {
                    if slot.group_id != group { break; }
                    if slot.state == SlotState::Pending {
                        slot.state = SlotState::Missed;
                    }
                }
                let space_pos = if boundary_pos == cursor {
                    self.test.slots.insert(boundary_pos, CharSlot {
                        expected: '\n',
                        kind: SlotKind::Regular,
                        group_id: group,
                        visual_width: 1,
                        state: SlotState::Extra('\n'),
                    });
                    boundary_pos + 1
                } else {
                    boundary_pos
                };
                if boundary_pos > cursor {
                    self.test.slots[space_pos].state = SlotState::Missed;
                } else {
                    self.test.slots[space_pos].state = SlotState::Wrong('\n');
                }
                self.test.live_incorrect_keystrokes += 1;
                self.test.total_errors_ever += 1;
                self.test.input.push(' ');
            }
            SlotKind::Newline => {
                let group = self.test.slots[boundary_pos].group_id;
                let line_has_error = self.test.slots[..=boundary_pos]
                    .iter()
                    .filter(|s| s.group_id == group)
                    .any(|s| matches!(s.state,
                        SlotState::Wrong(_) | SlotState::Uncommitted(_)
                            | SlotState::Missed | SlotState::Extra(_)
                    ));

                if boundary_pos > cursor {
                    self.test.slots[boundary_pos].state = SlotState::Missed;
                    self.test.live_incorrect_keystrokes += 1;
                    self.test.total_errors_ever += 1;
                    self.test.input.push('\n');
                } else if !line_has_error {
                    self.test.slots[boundary_pos].state = SlotState::Correct;
                    self.test.live_correct_keystrokes += 1;
                    self.test.input.push('\n');

                    let mut i = boundary_pos + 1;
                    while i < self.test.slots.len() {
                        if self.test.slots[i].kind == SlotKind::Tab {
                            self.test.slots[i].state = SlotState::Correct;
                            self.test.gross_char_count += 1;
                            self.test.live_correct_keystrokes += 1;
                            for _ in 0..self.test.slots[i].visual_width {
                                self.test.input.push('\t');
                            }
                            i += 1;
                        } else {
                            break;
                        }
                    }
                } else {
                    self.test.slots[boundary_pos].state = SlotState::Wrong('\n');
                    self.test.live_incorrect_keystrokes += 1;
                    self.test.total_errors_ever += 1;
                    self.test.input.push('\n');
                }
            }
            _ => {}
        }

        self.sync_display_text();
        self.check_scroll_trigger();
        self.check_test_completion();
    }



    pub fn on_tab(&mut self) {
        if self.test.state == AppState::Finished { return; }
        if self.test.state == AppState::Waiting {
            self.test.start_time = Some(std::time::Instant::now());
            self.test.state = AppState::Running;
        }

        let cursor = match self.test.slots.iter().position(|s| s.state == SlotState::Pending) {
            Some(i) => i,
            None    => return,
        };

        if let Some(glyph) = self.uncommitted_glyph() {
            if self.test.slots[glyph].kind == SlotKind::Tab {
                self.show_ui = false;
                self.test.gross_char_count += 1;
                self.test.live_correct_keystrokes += 1;
                self.test.slots[glyph].state = SlotState::Correct;
                for _ in 0..self.test.slots[glyph].visual_width {
                    self.test.input.push('\t');
                }
                self.sync_display_text();
                self.check_scroll_trigger();
                self.check_test_completion();
            }
            return;
        }

        if self.test.slots[cursor].kind == SlotKind::Tab {
            self.show_ui = false;
            self.test.gross_char_count += 1;
            self.test.live_correct_keystrokes += 1;
            self.test.slots[cursor].state = SlotState::Correct;

            for _ in 0..self.test.slots[cursor].visual_width {
                self.test.input.push('\t');
            }

            self.sync_display_text();
            self.check_scroll_trigger();
            self.check_test_completion();
        }
    }
}
