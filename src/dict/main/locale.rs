use crate::lang::Lang;

// This should be done differently, and support every section of the dictionary (i.e. Etymology)

/// Localize Example/Examples
///
/// In practice, this is only called for target: Edition, in the main dictionary
pub fn localize_examples_string(target: Lang, n: usize) -> String {
    let (singular, plural) = match target {
        Lang::Fr => ("exemple", "exemples"),
        Lang::De => ("Beispiel", "Beispiele"),
        Lang::Es => ("ejemplo", "ejemplos"),
        Lang::Ru => ("пример", "примеры"),
        Lang::Cs => ("příklad", "příklady"),
        Lang::Nl => ("voorbeeld", "voorbeelden"),
        Lang::El => ("παράδειγμα", "παραδείγματα"),
        Lang::It => ("esempio", "esempi"),
        Lang::Ku => ("nimûne", "nimûneyên"),
        Lang::Pl => ("przykład", "przykłady"),
        Lang::Pt => ("exemplo", "exemplos"),

        // no plural distinction
        Lang::Id | Lang::Ms => ("contoh", "contoh"),
        Lang::Tr => ("örnek", "örnek"),
        Lang::Zh | Lang::Ja => ("例", "例"),
        Lang::Ko => ("예문", "예문"),
        Lang::Th => ("ตัวอย่าง", "ตัวอย่าง"),
        Lang::Vi => ("ví dụ", "ví dụ"),

        _ => ("example", "examples"),
    };

    if n == 1 {
        format!("1 {singular}")
    } else {
        format!("{n} {plural}")
    }
}

/// Localize Etymology
pub const fn localize_etymology_string(target: Lang) -> &'static str {
    match target {
        Lang::Zh => "词源",
        Lang::Cs => "Etymologie",
        Lang::Nl => "Etymologie",
        Lang::Fr => "Étymologie",
        Lang::De => "Etymologie",
        Lang::El => "Ετυμολογία",
        Lang::Id => "Etimologi",
        Lang::It => "Etimologia",
        Lang::Ja => "語源",
        Lang::Ko => "어원",
        Lang::Ku => "Rêç",
        Lang::Ms => "Etimologi",
        Lang::Pl => "Etymologia",
        Lang::Pt => "Etimologia",
        Lang::Ru => "Этимология",
        Lang::Es => "Etimología",
        Lang::Th => "รากศัพท์",
        Lang::Tr => "Etimoloji",
        Lang::Vi => "Nguồn gốc từ",
        _ => "Etymology",
    }
}

/// Localize Grammar
pub const fn localize_grammar_string(target: Lang) -> &'static str {
    match target {
        Lang::Zh => "语法",
        Lang::Cs => "Gramatika",
        Lang::Nl => "Grammatica",
        Lang::Fr => "Grammaire",
        Lang::De => "Grammatik",
        Lang::El => "Γραμματική",
        Lang::Id => "Tata bahasa",
        Lang::It => "Grammatica",
        Lang::Ja => "文法",
        Lang::Ko => "문법",
        Lang::Ku => "Gramera",
        Lang::Ms => "Tatabahasa",
        Lang::Pl => "Gramatyka",
        Lang::Pt => "Gramática",
        Lang::Ru => "Грамматика",
        Lang::Es => "Gramática",
        Lang::Th => "ไวยากรณ์",
        Lang::Tr => "Dilbilgisi",
        Lang::Vi => "Ngữ pháp",
        _ => "Grammar",
    }
}

/// Localize Synonyms
pub const fn localize_synonyms_string(target: Lang) -> &'static str {
    match target {
        Lang::Zh => "同义词",
        Lang::Cs => "Synonyma",
        Lang::Nl => "Synoniemen",
        Lang::Fr => "Synonymes",
        Lang::De => "Synonyme",
        Lang::El => "Συνώνυμα",
        Lang::Id => "Sinonim",
        Lang::It => "Sinonimi",
        Lang::Ja => "類義語",
        Lang::Ko => "동의어",
        Lang::Ku => "Hevwate",
        Lang::Ms => "Sinonim",
        Lang::Pl => "Synonimy",
        Lang::Pt => "Sinónimos",
        Lang::Ru => "Синонимы",
        Lang::Es => "Sinónimos",
        Lang::Th => "คำพ้องความหมาย",
        Lang::Tr => "Eş anlamlılar",
        Lang::Vi => "Từ đồng nghĩa",
        _ => "Synonyms",
    }
}

/// Localize Antonyms
pub const fn localize_antonyms_string(target: Lang) -> &'static str {
    match target {
        Lang::Zh => "反义词",
        Lang::Cs => "Antonyma",
        Lang::Nl => "Antoniemen",
        Lang::Fr => "Antonymes",
        Lang::De => "Antonyme",
        Lang::El => "Αντώνυμα",
        Lang::Id => "Antonim",
        Lang::It => "Contrari",
        Lang::Ja => "対義語",
        Lang::Ko => "반의어",
        Lang::Ms => "Antonim",
        Lang::Pl => "Antonimy",
        Lang::Pt => "Antónimos",
        Lang::Ru => "Антонимы",
        Lang::Es => "Antónimos",
        Lang::Th => "คำตรงข้าม",
        Lang::Tr => "Zıt anlamlılar",
        Lang::Vi => "Từ trái nghĩa",
        _ => "Antonyms",
    }
}
