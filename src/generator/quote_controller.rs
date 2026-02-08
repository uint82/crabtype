use crate::models::{QuoteData, QuoteSelector};
use super::sourcing::TextSource;

pub struct QuoteResult {
    pub word_stream: Vec<String>,
    pub quote_pool: Vec<String>,
    pub total_words: usize,
    pub source_text: String,
}

pub fn generate(
    source: &TextSource,
    selector: &QuoteSelector,
    quote_data: &QuoteData,
    rng: &mut impl rand::Rng,
) -> QuoteResult {
    if let Some((all_words, quote_source)) = source.get_quote_text(selector, quote_data, rng) {
        let total_words = all_words.len();
        let (word_stream, quote_pool) = if all_words.len() > 100 {
            let stream = all_words[..100].to_vec();
            let mut pool = all_words[100..].to_vec();
            pool.reverse();
            (stream, pool)
        } else {
            (all_words, Vec::new())
        };

        QuoteResult {
            word_stream,
            quote_pool,
            total_words,
            source_text: quote_source,
        }
    } else {
        QuoteResult {
            word_stream: vec!["No".to_string(), "Quote".to_string(), "Found".to_string()],
            quote_pool: Vec::new(),
            total_words: 3,
            source_text: "System".to_string(),
        }
    }
}

pub fn next_word(quote_pool: &mut Vec<String>) -> Option<Vec<String>> {
    quote_pool.pop().map(|w| vec![w])
}
