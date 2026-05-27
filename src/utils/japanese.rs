/// Check if the character is Kanji.
pub const fn is_kanji(c: char) -> bool {
    matches!(c, '\u{4E00}'..='\u{9FFF}'
        | '\u{3400}'..='\u{4DBF}'
        | '\u{F900}'..='\u{FAFF}'
        | '\u{20000}'..='\u{2A6DF}'
        | '\u{2A700}'..='\u{2CEAF}'
        | '\u{2CEB0}'..='\u{2EBEF}'
    )
}

/// Check if the word has any Kanji.
pub fn has_kanji(word: &str) -> bool {
    word.chars().any(is_kanji)
}
