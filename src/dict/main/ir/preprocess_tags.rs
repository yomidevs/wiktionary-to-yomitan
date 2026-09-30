//! Preprocesses the tags of an entry, its forms and its senses.

use crate::{
    lang::{Edition, Lang},
    models::kaikki::WordEntry,
};

pub fn preprocess_tags(edition: Edition, source: Lang, entry: &mut WordEntry) {
    if (edition, source) == (Edition::Ja, Lang::Ja) {
        preprocess_tags_ja_ja(entry);
    }
}

fn preprocess_tags_ja_ja(entry: &mut WordEntry) {
    let forms = entry.forms.iter_mut().map(|form| &mut form.tags);
    let senses = entry.senses.iter_mut().map(|sense| &mut sense.tags);
    for tags in std::iter::once(&mut entry.tags).chain(forms).chain(senses) {
        if tags.iter().any(|tag| tag == "ichidan") {
            tags.retain(|tag| tag != "shimoichidan" && tag != "kamiichidan");
        }
    }
}
