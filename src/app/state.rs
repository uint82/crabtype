use crate::config::Theme;
use crate::models::{AppState, Mode, QuoteData, Word, WordData};
use crate::generator::WordGenerator;
use std::collections::{HashMap, HashSet};
use std::time::Instant;

#[derive(Clone, Debug, PartialEq)]
pub enum SlotKind {
    Regular,
    Space,
    Newline,
    Tab,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SlotState {
    Pending,
    Correct,
    Wrong(char),
    Uncommitted(char),
    Extra(char),
    Missed,
}

#[derive(Clone, Debug)]
pub struct CharSlot {
    pub expected: char,
    pub kind: SlotKind,
    pub group_id: usize,
    pub visual_width: u8,
    pub state: SlotState,
}

pub struct SessionConfig {
    pub mode: Mode,
    pub theme: Theme,
    pub use_numbers: bool,
    pub use_punctuation: bool,
    pub word_data: WordData,
    pub quote_data: Option<QuoteData>,
    pub(crate) word_generator: WordGenerator,
}

pub struct TestState {
    pub state: AppState,

    pub slots: Vec<CharSlot>,

    pub input: String,
    pub cursor_idx: usize,
    pub start_time: Option<Instant>,

    pub gross_char_count: usize,
    pub total_errors_ever: usize,
    pub processed_word_errors: HashSet<usize>,

    pub generated_count: usize,
    pub scrolled_word_count: usize,
    pub furthest_word_idx: usize,

    pub st_correct: usize,
    pub st_incorrect: usize,
    pub st_extra: usize,
    pub st_missed: usize,

    pub acc_score_correct: isize,
    pub acc_score_incorrect: isize,

    pub uncorrected_errors_scrolled: usize,

    pub live_correct_keystrokes: usize,
    pub live_incorrect_keystrokes: usize,

    pub final_wpm: f64,
    pub final_raw_wpm: f64,
    pub final_accuracy: f64,
    pub final_consistency: f64,
    pub final_time: f64,

    pub current_quote_source: String,

    pub word_stream: Vec<Word>,
    pub word_stream_string: String,

    pub visual_lines: Vec<String>,
    pub display_string: String,
    pub display_mask: Vec<u8>,
    pub display_breaks: Vec<bool>,
    pub extra_char_count: usize,

    pub missed_chars: HashMap<usize, usize>,
    pub aligned_input: Vec<char>,

    pub quote_pool: Vec<String>,
    pub total_quote_words: usize,
    pub total_code_words: usize,
    pub code_words_scrolled: usize,
    pub original_quote_length: usize,
    pub next_word_index: usize,

    pub is_new_best: bool,

    pub caret_epoch: Instant,

    pub cumulative_words: Vec<String>,
    pub overflow_pool: Vec<String>,

    pub wpm_history: Vec<(f64, f64)>,
    pub raw_wpm_history: Vec<(f64, f64)>,
    pub errors_history: Vec<(f64, f64)>,
    pub(crate) last_snapshot_second: u64,
    pub(crate) prev_incorrect_keystrokes: usize,
    pub(crate) prev_gross_char_count: usize,

    pub burst_wpm_history: Vec<f64>,
}

impl Default for TestState {
    fn default() -> Self {
        Self {
            slots: Vec::new(),
            state: AppState::Waiting,
            input: String::new(),
            cursor_idx: 0,
            start_time: None,
            gross_char_count: 0,
            total_errors_ever: 0,
            processed_word_errors: HashSet::new(),
            generated_count: 0,
            scrolled_word_count: 0,
            furthest_word_idx: 0,
            st_correct: 0,
            st_incorrect: 0,
            st_extra: 0,
            st_missed: 0,
            acc_score_correct: 0,
            acc_score_incorrect: 0,
            uncorrected_errors_scrolled: 0,
            live_correct_keystrokes: 0,
            live_incorrect_keystrokes: 0,
            final_wpm: 0.0,
            final_raw_wpm: 0.0,
            final_accuracy: 0.0,
            final_consistency: 0.0,
            final_time: 0.0,
            current_quote_source: String::new(),
            word_stream: Vec::new(),
            word_stream_string: String::new(),
            visual_lines: Vec::new(),
            display_string: String::new(),
            display_mask: Vec::new(),
            display_breaks: Vec::new(),
            extra_char_count: 0,
            missed_chars: HashMap::new(),
            aligned_input: Vec::new(),
            quote_pool: Vec::new(),
            total_quote_words: 0,
            total_code_words: 0,
            code_words_scrolled: 0,
            original_quote_length: 0,
            next_word_index: 0,
            is_new_best: false,
            caret_epoch: Instant::now(),
            cumulative_words: Vec::new(),
            overflow_pool: Vec::new(),
            wpm_history: Vec::new(),
            raw_wpm_history: Vec::new(),
            errors_history: Vec::new(),
            last_snapshot_second: u64::MAX,
            prev_incorrect_keystrokes: 0,
            prev_gross_char_count: 0,
            burst_wpm_history: Vec::new(),
        }
    }
}

pub struct App {
    pub should_quit: bool,
    pub show_ui: bool,
    pub terminal_width: u16,
    pub last_test_words: Option<Vec<String>>,

    pub config: SessionConfig,
    pub test: TestState,
    pub discord: Option<crate::discord::DiscordPresence>,
}

