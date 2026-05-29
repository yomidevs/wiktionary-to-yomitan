/// Check if the character is kana.
pub const fn is_kana_c(c: char) -> bool {
    matches!(c, '\u{3040}'..='\u{309F}' | '\u{30A0}'..='\u{30FF}')
}

/// Check if the string is all kana.
pub fn is_kana(s: &str) -> bool {
    s.chars().all(is_kana_c)
}

/// Check if the character is kanji.
pub const fn is_kanji_c(c: char) -> bool {
    matches!(c, '\u{4E00}'..='\u{9FFF}'
        | '\u{3400}'..='\u{4DBF}'
        | '\u{F900}'..='\u{FAFF}'
        | '\u{20000}'..='\u{2A6DF}'
        | '\u{2A700}'..='\u{2CEAF}'
        | '\u{2CEB0}'..='\u{2EBEF}'
    )
}

/// Check if the string has any kanji.
pub fn has_kanji(s: &str) -> bool {
    s.chars().any(is_kanji_c)
}
