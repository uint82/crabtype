use crate::models::{QuoteData, QuoteEntry, QuoteLength, QuoteSelector, WordData};
use crate::utils::strings;
use rand::prelude::IndexedRandom;
use rand::seq::SliceRandom;
use rand::Rng;

pub struct TextSource {
    word_data: WordData,
}

impl TextSource {
    pub fn new(word_data: WordData) -> Self {
        Self { word_data }
    }

    pub fn get_random_word(&self, rng: &mut impl Rng) -> String {
        self.word_data
            .words
            .choose(rng)
            .cloned()
            .unwrap_or_else(|| "word".to_string())
    }

    pub fn get_unique_batch(&self, count: usize, rng: &mut impl Rng) -> Vec<String> {
        let mut deck = self.word_data.words.clone();
        deck.shuffle(rng);
        deck.into_iter().take(count).collect()
    }

    pub fn get_quote_text(
        &self,
        selector: &QuoteSelector,
        quote_data: &QuoteData,
        rng: &mut impl Rng,
    ) -> Option<(Vec<String>, String)> {
        let q_opt = match selector {
            QuoteSelector::Id(target_id) => quote_data.quotes.iter().find(|q| q.id == *target_id),
            QuoteSelector::Category(len_category) => {
                let range = match len_category {
                    QuoteLength::Short => &quote_data.groups[0],
                    QuoteLength::Medium => &quote_data.groups[1],
                    QuoteLength::Long => &quote_data.groups[2],
                    QuoteLength::VeryLong => &quote_data.groups[3],
                    QuoteLength::All => &vec![0, 9999],
                };
                let valid: Vec<&QuoteEntry> = quote_data
                    .quotes
                    .iter()
                    .filter(|q| q.length >= range[0] && q.length <= range[1])
                    .collect();

                valid.choose(rng).copied()
            }
        };

        if let Some(q) = q_opt {
            let clean_text = strings::clean_typography_symbols(&q.text);
            let all_words: Vec<String> = clean_text.split_whitespace().map(String::from).collect();
            Some((all_words, q.source.clone()))
        } else {
            None
        }
    }
}
