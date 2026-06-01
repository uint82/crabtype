use crate::models::Mode;
use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const GITHUB_URL: &str = "https://github.com/uint82/typa";

pub struct DiscordPresence {
    client: DiscordIpcClient,
    pub connected: bool,
    start_timestamp: i64,
    last_activity_call: Option<Instant>,
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

fn mode_label(mode: &Mode, use_punctuation: bool, use_numbers: bool, quote_length: &str, quote_source: &str, language: &str) -> String {
    match mode {
        Mode::Time(t)  => format_with_mods(format!("Time {}s", t), language, use_punctuation, use_numbers),
        Mode::Words(w) => format_with_mods(format!("Words {}", w), language, use_punctuation, use_numbers),
        Mode::Quote(_) => {
            let base = if quote_length.is_empty() {
                "Quote".to_string()
            } else {
                format!("Quote {}", quote_length)
            };
            let with_lang   = if language.is_empty()     { base }      else { format!("{} {}", base, language) };
            let with_source = if quote_source.is_empty() { with_lang } else { format!("{} · {}", with_lang, quote_source) };
            with_source
        }
    }
}

fn format_with_mods(base: String, language: &str, use_punctuation: bool, use_numbers: bool) -> String {
    let mut parts: Vec<String> = vec![base];
    if !language.is_empty() { parts.push(language.to_string()); }
    if use_punctuation      { parts.push("punc".to_string()); }
    if use_numbers          { parts.push("num".to_string()); }
    parts.join(" ")
}

impl DiscordPresence {
    pub fn new() -> Self {
        let start_timestamp = now_unix();
        let mut client = DiscordIpcClient::new("1497293795988078672")
            .expect("Failed to create Discord client");
        let connected = client.connect().is_ok();
        Self { client, connected, start_timestamp, last_activity_call: None }
    }

    fn reconnect(&mut self) -> bool {
        let _ = self.client.close();
        self.connected = self.client.connect().is_ok();
        self.connected
    }

    fn set_activity_with_retry(&mut self, act: activity::Activity<'_>, force: bool) {
        if !self.connected { return; }
        if !force {
            if let Some(last) = self.last_activity_call {
                if last.elapsed().as_millis() < 1000 { return; }
            }
        }
        self.last_activity_call = Some(Instant::now());
        if self.client.set_activity(act.clone()).is_err() {
            if self.reconnect() {
                let _ = self.client.set_activity(act);
            }
        }
    }

    pub fn set_idle(&mut self, mode: &Mode, use_punctuation: bool, use_numbers: bool, quote_length: &str, language: &str) {
        self.set_activity_with_retry(
            activity::Activity::new()
                .details(&mode_label(mode, use_punctuation, use_numbers, quote_length, "", language))
                .state("Waiting to type...")
                .assets(
                    activity::Assets::new()
                        .large_image("typa_logo")
                        .large_text("typa"),
                )
                .timestamps(
                    activity::Timestamps::new().start(self.start_timestamp)
                )
                .buttons(vec![
                    activity::Button::new("GitHub", GITHUB_URL),
                ])
        , false);
    }

    pub fn set_typing(&mut self, mode: &Mode, use_punctuation: bool, use_numbers: bool, quote_length: &str, language: &str) {
        self.set_activity_with_retry(
            activity::Activity::new()
                .details(&mode_label(mode, use_punctuation, use_numbers, quote_length, "", language))
                .state("Typing...")
                .assets(
                    activity::Assets::new()
                        .large_image("typa_logo")
                        .large_text("typa"),
                )
                .timestamps(
                    activity::Timestamps::new().start(self.start_timestamp)
                )
                .buttons(vec![
                    activity::Button::new("GitHub", GITHUB_URL),
                ])
        , false);
    }

    pub fn set_stats(&mut self, best_wpm: f64, total_tests: usize, current_streak: usize) {
        let details = if current_streak > 0 {
            format!("In stats | {} day streak", current_streak)
        } else {
            "Browsing stats".to_string()
        };
        let state = format!("{:.0} WPM best | {} tests", best_wpm, total_tests);
        self.set_activity_with_retry(
            activity::Activity::new()
                .details(&details)
                .state(&state)
                .assets(
                    activity::Assets::new()
                        .large_image("typa_logo")
                        .large_text("typa"),
                )
                .timestamps(
                    activity::Timestamps::new().start(self.start_timestamp)
                )
                .buttons(vec![
                    activity::Button::new("GitHub", GITHUB_URL),
                ])
        , false);
    }

    pub fn set_result(
        &mut self,
        wpm: f64,
        accuracy: f64,
        mode: &Mode,
        is_new_best: bool,
        _typed_words: usize,
        _total_words: usize,
        quote_source: &str,
        consistency: f64,
        use_punctuation: bool,
        use_numbers: bool,
        quote_length: &str,
        language: &str,
    ) {
        let details = format!("{:.0} WPM | {:.1}% acc | {:.0}% con", wpm, accuracy, consistency);

        let mode_str = mode_label(mode, use_punctuation, use_numbers, quote_length, quote_source, language);

        let state = if is_new_best { format!("New best!  {}", mode_str) } else { mode_str };

        self.set_activity_with_retry(
            activity::Activity::new()
                .details(&details)
                .state(&state)
                .assets(
                    activity::Assets::new()
                        .large_image("typa_logo")
                        .large_text("typa"),
                )
                .timestamps(
                    activity::Timestamps::new().start(self.start_timestamp)
                )
                .buttons(vec![
                    activity::Button::new("GitHub", GITHUB_URL),
                ])
        , false);
    }

}

impl Drop for DiscordPresence {
    fn drop(&mut self) {
        if self.connected {
            let _ = self.client.close();
        }
    }
}
