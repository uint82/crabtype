use super::state::{App, SessionConfig, TestState};
use crate::config::Theme;
use crate::history;
use crate::models::{AppState, Mode, QuoteData, WordData, Word, WordState};
use crate::generator::WordGenerator;
use anyhow::{Context, Result};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "resources/"]
struct Asset;

impl App {
    pub fn new(
        mode: Mode,
        language: String,
        use_numbers: bool,
        use_punctuation: bool,
        theme: Theme,
    ) -> Result<Self> {
        let word_data: WordData = if !matches!(mode, Mode::Quote(_)) {
            let word_filename = format!("language/{}.json", language);
            let word_file = Asset::get(&word_filename).context(format!(
                "Could not find embedded language file: {}", word_filename
            ))?;
            serde_json::from_str(std::str::from_utf8(word_file.data.as_ref())?)?
        } else {
            WordData { name: language.clone(), words: Vec::new() }
        };

        let quote_data: Option<QuoteData> = if matches!(mode, Mode::Quote(_)) {
            let quote_filename = format!("quotes/{}.json", language);
            let quote_file = Asset::get(&quote_filename).context(format!(
                "Could not find embedded quotes file: {}", quote_filename
            ))?;
            Some(serde_json::from_str(std::str::from_utf8(quote_file.data.as_ref())?)?)
        } else {
            None
        };

        let word_generator = WordGenerator::new(
            word_data.clone(),
            use_numbers,
            use_punctuation,
        );

        let config = SessionConfig {
            mode,
            theme,
            use_numbers,
            use_punctuation,
            word_data,
            quote_data,
            word_generator,
        };

        let mut app = Self {
            should_quit: false,
            show_ui: true,
            terminal_width: 80,
            last_test_words: None,
            config,
            test: TestState::default(),
            discord: None,
        };

        app.generate_initial_words();

        let mut dp = crate::discord::DiscordPresence::new();
        if dp.connected {
            use crate::ui::utils::quote_idle_label;
            let ql = match &app.config.mode {
                Mode::Quote(q) => quote_idle_label(q, app.test.original_quote_length),
                _ => "",
            };
            dp.set_idle(&app.config.mode, app.config.use_punctuation, app.config.use_numbers, ql, &app.config.word_data.name);
            app.discord = Some(dp);
        } else {
            app.discord = None;
        }

        Ok(app)
    }

    pub fn quit(&mut self) {
        if self.test.state == AppState::Running {
            let _ = history::record_test(self, false);
        }
        self.should_quit = true;
    }

    pub fn restart_test(&mut self) {
        if self.test.state == AppState::Running {
            let _ = history::record_test(self, false);
        }
        if !self.test.cumulative_words.is_empty() {
            self.last_test_words = Some(self.test.cumulative_words.clone());
        }
        self.test = TestState::default();
        self.show_ui = true;
        self.generate_initial_words();
        if let Some(ref mut d) = self.discord {
            use crate::ui::utils::quote_idle_label;
            let ql = match &self.config.mode {
                Mode::Quote(q) => quote_idle_label(q, self.test.original_quote_length),
                _ => "",
            };
            d.set_idle(&self.config.mode, self.config.use_punctuation, self.config.use_numbers, ql, &self.config.word_data.name);
        }
    }

    pub fn retry_last_test(&mut self) {
        let words = match self.last_test_words.clone() {
            Some(w) if !w.is_empty() => w,
            _ => { self.restart_test(); return; }
        };
        if self.test.state == AppState::Running {
            let _ = history::record_test(self, false);
        }
        self.test = TestState::default();
        self.show_ui = true;
        self.seed_from_word_list(words);
    }

    pub(crate) fn seed_from_word_list(&mut self, words: Vec<String>) {
        let total = words.len();
        let cap   = 100.min(total);

        let word_stream: Vec<Word> = words[..cap].iter().enumerate().map(|(i, text)| {
            let mut w = Word::new(text.clone(), i);
            if i == 0 { w.state = WordState::Active; }
            w
        }).collect();

        let mut overflow: Vec<String> = words[cap..].to_vec();
        overflow.reverse();

        self.test.word_stream     = word_stream;
        self.test.generated_count = total;
        self.test.next_word_index = cap;
        self.test.cumulative_words = words;
        self.test.overflow_pool    = overflow;

        if matches!(self.config.mode, Mode::Quote(_)) {
            self.test.total_quote_words = total;
        }

        self.update_stream_string();
        self.sync_display_text();

        if matches!(self.config.mode, Mode::Quote(_)) {
            self.test.original_quote_length = self.test.word_stream_string.chars().count();
        }

        self.recalculate_lines();
    }
}
