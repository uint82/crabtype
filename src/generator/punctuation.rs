use rand::prelude::IndexedRandom;
use rand::Rng;

pub struct PunctuationRules {
    pub use_punctuation: bool,
    pub use_numbers: bool,
}

impl PunctuationRules {
    pub fn apply(&self, mut word: String, rng: &mut impl Rng) -> String {
        if self.use_numbers && rng.random_bool(0.15) {
            return rng.random_range(0..=9999).to_string();
        }

        if !self.use_punctuation {
            return word;
        }

        if rng.random_bool(0.40) {
            word = self.apply_contraction(&word, rng);
        }

        if rng.random_bool(0.30) {
            let p_type = rng.random_range(0..100);
            match p_type {
                0..=39 => word.push(','),
                40..=69 => word.push('.'),
                70..=74 => word.push('!'),
                80..=89 => word = format!("\"{}\"", word),
                90..=94 => word = format!("'{}'", word),
                95..=99 => word = format!("({})", word),
                _ => {}
            }
        }
        word
    }

    pub fn should_insert_dash(&self, rng: &mut impl Rng) -> bool {
        self.use_punctuation && rng.random_bool(0.30) && rng.random_range(75..=79) == 75
    }

    fn apply_contraction(&self, original: &str, rng: &mut impl Rng) -> String {
        let lower = original.to_lowercase();
        if let Some(replacements) = self.get_contraction_replacements(&lower) {
            if let Some(replacement) = replacements.choose(rng) {
                return self.match_casing(original, replacement);
            }
        }
        original.to_string()
    }

    fn match_casing(&self, original: &str, replacement: &str) -> String {
        let is_all_upper = original
            .chars()
            .all(|c| !c.is_alphabetic() || c.is_uppercase());

        if is_all_upper {
            return replacement.to_uppercase();
        }

        let first_is_upper = original.chars().next().map_or(false, |c| c.is_uppercase());
        if first_is_upper {
            let mut c = replacement.chars();
            match c.next() {
                Option::None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        } else {
            replacement.to_string()
        }
    }

    fn get_contraction_replacements(&self, word: &str) -> Option<&'static [&'static str]> {
        match word {
            "are" => Some(&["aren't"]),
            "can" => Some(&["can't"]),
            "could" => Some(&["couldn't"]),
            "did" => Some(&["didn't"]),
            "does" => Some(&["doesn't"]),
            "do" => Some(&["don't"]),
            "had" => Some(&["hadn't"]),
            "has" => Some(&["hasn't"]),
            "have" => Some(&["haven't"]),
            "is" => Some(&["isn't"]),
            "it" => Some(&["it's", "it'll"]),
            "i" => Some(&["i'm", "i'll", "i've", "i'd"]),
            "you" => Some(&["you'll", "you're", "you've", "you'd"]),
            "that" => Some(&["that's", "that'll", "that'd"]),
            "must" => Some(&["mustn't", "must've"]),
            "there" => Some(&["there's", "there'll", "there'd"]),
            "he" => Some(&["he's", "he'll", "he'd"]),
            "she" => Some(&["she's", "she'll", "she'd"]),
            "we" => Some(&["we're", "we'll", "we'd"]),
            "they" => Some(&["they're", "they'll", "they'd"]),
            "should" => Some(&["shouldn't", "should've"]),
            "was" => Some(&["wasn't"]),
            "were" => Some(&["weren't"]),
            "will" => Some(&["won't"]),
            "would" => Some(&["wouldn't", "would've"]),
            "going" => Some(&["goin'"]),
            _ => None,
        }
    }
}
