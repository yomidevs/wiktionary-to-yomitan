//! Post-processing pass.
//!
//! Deduplicates and normalizes tags across all forms.

use crate::{
    Set,
    cli::LangSpecs,
    dict::main::ir::{FormMap, LemmaMap, Tidy},
    lang::Lang,
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

    // TODO: implement TryFrom<LangSpecs> for Lang, and use edition here
    // also match on the language pair for safety
    if matches!(langs.target, Lang::Ja) {
        postprocess_japanese_kanji_lemmas(irs);
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

fn postprocess_japanese_kanji_lemmas(irs: &mut Tidy) {
    let kana_to_kanji: Vec<_> = irs
        .form_map
        .flat_iter()
        .filter(|(_, _, _, _, tags)| tags.iter().any(|t| t == "kanji"))
        .map(|(uninflected, inflected, _, _, _)| (uninflected.to_string(), inflected.to_string()))
        .collect();

    let mut new_lemmas = LemmaMap::default();
    for (kana, kanji) in &kana_to_kanji {
        for (lemma, reading, pos, info) in irs.lemma_map.flat_iter() {
            if lemma == kana || reading == kana {
                new_lemmas.insert(kanji, kana, pos.short(), info.clone());
                break;
            }
        }
    }

    let n_forms_promoted = new_lemmas.len();
    for (key, infos) in new_lemmas.0 {
        let (kanji, kana_reading, pos) = key.unpack();
        for info in infos {
            irs.lemma_map.insert(kanji, kana_reading, pos.short(), info);
        }
    }

    // Remove forms that were just promoted to lemmas
    let promoted: Set<&str> = kana_to_kanji
        .iter()
        .map(|(_, kanji)| kanji.as_str())
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
