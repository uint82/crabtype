use ratatui::style::Color;

pub fn hex_to_rgb(hex: &str) -> Color {
    let hex = hex.trim_start_matches('#');
    if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255);
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255);
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(255);
        Color::Rgb(r, g, b)
    } else {
        Color::White
    }
}

pub fn format_timer(seconds: u64) -> String {
    if seconds >= 60 {
        let minutes = seconds / 60;
        let secs = seconds % 60;
        format!("{}:{:02}", minutes, secs)
    } else {
        format!("{}", seconds)
    }
}

pub fn get_quote_length_category(char_count: usize) -> &'static str {
    if char_count <= 100 {
        "short"
    } else if char_count <= 300 {
        "medium"
    } else if char_count <= 600 {
        "long"
    } else {
        "very long"
    }
}
