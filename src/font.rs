//! A tiny block font for drawing the countdown in large digits.

pub const HEIGHT: usize = 5;

fn glyph(c: char) -> [&'static str; HEIGHT] {
    match c {
        '0' => ["█████", "█   █", "█   █", "█   █", "█████"],
        '1' => ["  █  ", " ██  ", "  █  ", "  █  ", " ███ "],
        '2' => ["█████", "    █", "█████", "█    ", "█████"],
        '3' => ["█████", "    █", " ████", "    █", "█████"],
        '4' => ["█   █", "█   █", "█████", "    █", "    █"],
        '5' => ["█████", "█    ", "█████", "    █", "█████"],
        '6' => ["█████", "█    ", "█████", "█   █", "█████"],
        '7' => ["█████", "    █", "   █ ", "  █  ", "  █  "],
        '8' => ["█████", "█   █", "█████", "█   █", "█████"],
        '9' => ["█████", "█   █", "█████", "    █", "█████"],
        ':' => ["   ", " █ ", "   ", " █ ", "   "],
        _ => ["     "; HEIGHT],
    }
}

/// Renders `text` as `HEIGHT` lines of block characters.
pub fn render(text: &str) -> Vec<String> {
    (0..HEIGHT)
        .map(|row| {
            text.chars()
                .map(|c| glyph(c)[row])
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect()
}
