use crate::utils::strings;

pub fn apply_contextual_capitalization(
    new_words: &mut [String],
    existing_stream: &[String],
    use_punctuation: bool,
) {
    if !use_punctuation {
        return;
    }
    if let Some(first_new) = new_words.first_mut() {
        if let Some(last_existing) = existing_stream.last() {
            if strings::ends_with_terminator(last_existing) {
                strings::capitalize_word(first_new);
            }
        }
    }
}

pub fn finalize_stream_punctuation(stream: &mut Vec<String>) {
    if stream.is_empty() { return; }

    if let Some(first) = stream.first_mut() {
        strings::capitalize_word(first);
    }

    let len = stream.len();
    for i in 0..len - 1 {
        if strings::ends_with_terminator(&stream[i]) {
            strings::capitalize_word(&mut stream[i + 1]);
        }
    }

    if let Some(last) = stream.last_mut() {
        if last == "-" {
            *last = String::new();
        }
        if last.ends_with(',') {
            last.pop();
        }
        let c = last.chars().last().unwrap_or(' ');
        if !['.', '!', '?'].contains(&c) && !last.is_empty() {
            last.push('.');
        }
    }

    stream.retain(|s| !s.is_empty());
}
