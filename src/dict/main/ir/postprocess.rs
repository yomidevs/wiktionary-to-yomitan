//! Post-processing pass.
//!
//! Deduplicates and normalizes tags across all forms.

use crate::{
    Map, Set,
    cli::LangSpecs,
    dict::main::ir::{FormMap, LemmaMap, Tidy},
    lang::{Edition, Lang},
    tags::{
        merge_tags_by_case, merge_tags_by_definitiveness, merge_tags_by_gender,
        merge_tags_by_german_verb_type, merge_tags_by_person, merge_tags_by_verb_form,
        remove_redundant_tags, sort_tags, sort_tags_by_similar,
    },
};

pub fn postprocess_main(langs: LangSpecs, irs: &mut Tidy) {
    postprocess_forms(&mut irs.form_map);

    // Check for form redirects A > B where B does not have a lemma, to remove bloat.
    // This can happen when:
    // 1. A form redirects to another form that's not registered as a lemma
    // 2. Data inconsistencies in the source dictionary
    //
    // Caveats:
    // 1. People using multiple dictionaries, where B as a lemma in another dict.
    // 2. A > B > C and C has a lemma (to test)
    // check_orphaned_redirects(irs);

    // SAFETY: for the main dictionary, EditionSpec is always of the One variant.
    // This cast is only for convenience, we could match on EditionSpec variants directly.
    let edition: Edition = langs.edition.try_into().unwrap();
    match (edition, langs.source) {
        (Edition::Ja, Lang::Ja) => {
            let kana_to_kanji = collect_kana_to_kanji(&irs.form_map);
            postprocess_japanese_kanji_lemmas(irs, &kana_to_kanji);
        }
        _ => (),
    }
}

// For now, only diagnostic.
#[allow(unused)]
fn check_orphaned_redirects(irs: &mut Tidy) {
    let mut orphaned_count = 0;
    let total = irs.form_map.len();

    let lemmas_found: Set<_> = irs
        .lemma_map
        .0
        .iter()
        .map(|(key, _)| key.lemma.as_str())
        .collect();

    for (uninfl, _, _, _, _) in irs.form_map.flat_iter() {
        if !lemmas_found.contains(uninfl) {
            // tracing::debug!("{:?} does not exist as lemma", uninfl);
            orphaned_count += 1;
        }
    }

    tracing::error!("{orphaned_count} orphaned_count from {total}");
}

fn postprocess_forms(form_map: &mut FormMap) {
    for (_, _, _, _, tags) in form_map.flat_iter_mut() {
        // Keep only unique tags and remove tags subsets
        remove_redundant_tags(tags);

        // Merges
        // Note that while some of the merges are only relevant for certain editions,
        // they are quite cheap, and don't deserve (for now), to be only applied in case
        // we match some (Edition, Lang) pairs.
        merge_tags_by_person(tags);
        merge_tags_by_case(tags);
        merge_tags_by_verb_form(tags);
        merge_tags_by_definitiveness(tags); // [ko-en]
        merge_tags_by_gender(tags);
        merge_tags_by_german_verb_type(tags);

        // Sort inner words
        for tag in tags.iter_mut() {
            let mut words: Vec<&str> = tag.split(' ').collect();
            sort_tags(&mut words);
            *tag = words.join(" ");
        }

        sort_tags_by_similar(tags);
    }
}

fn collect_kana_to_kanji(form_map: &FormMap) -> Map<String, Vec<String>> {
    let mut map: Map<String, Vec<String>> = Map::default();
    for (uninflected, inflected, _, _, tags) in form_map.flat_iter() {
        if tags.iter().any(|t| t == "kanji") {
            map.entry(uninflected.to_string())
                .or_default()
                .push(inflected.to_string());
        }
    }
    map
}

fn postprocess_japanese_kanji_lemmas(irs: &mut Tidy, kana_to_kanji: &Map<String, Vec<String>>) {
    let mut new_lemmas = LemmaMap::default();
    for (lemma, reading, pos, info) in irs.lemma_map.flat_iter() {
        let kanji_writings = kana_to_kanji
            .get(lemma)
            .or_else(|| kana_to_kanji.get(reading));
        if let Some(kanjis) = kanji_writings {
            for kanji in kanjis {
                new_lemmas.insert(kanji, lemma, pos.long(), info.clone());
            }
        }
    }

    let n_forms_promoted = new_lemmas.len();
    for (key, infos) in new_lemmas.0 {
        let (kanji, kana_reading, pos) = key.unpack();
        for info in infos {
            irs.lemma_map.insert(kanji, kana_reading, pos.long(), info);
        }
    }

    // Remove forms that were just promoted to lemmas
    let promoted: Set<&str> = kana_to_kanji
        .values()
        .flatten()
        .map(String::as_str)
        .collect();
    let lemmas: Set<&str> = irs
        .lemma_map
        .0
        .iter()
        .map(|(key, _)| key.lemma.as_str())
        .collect();

    let n_forms_before = irs.form_map.len();
    irs.form_map.0.retain(|key, (_, tags)| {
        // Remove promoted kanji entries entirely
        if promoted.contains(key.uninflected.as_str()) {
            return false;
        }
        // Remove kanji > kana redirections only if the uninflected kanji has a lemma.
        // That is, remove redirection tags, and the form itself if it has no tags.
        // ("redirected from" are from "form-of" of wagokanji templates)
        if lemmas.contains(key.uninflected.as_str()) {
            tags.retain(|tag| tag != "kanji" && !tag.starts_with("redirected from"));
        }
        !tags.is_empty()
    });
    let n_forms_removed = n_forms_before - irs.form_map.len();

    tracing::debug!(
        "[ja] kanji postprocess: {n_forms_promoted} forms promoted to lemmas, {n_forms_removed} forms removed"
    );
}
