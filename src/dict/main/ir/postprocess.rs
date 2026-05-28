//! Post-processing pass.
//!
//! Deduplicates and normalizes tags across all forms.

use crate::{
    Map, Set,
    cli::LangSpecs,
    dict::main::ir::{FormMap, FormSource, LemmaMap, Tidy, found_ir_message_impl},
    lang::{Edition, Lang},
    tags::{
        merge_tags_by_case, merge_tags_by_definitiveness, merge_tags_by_gender,
        merge_tags_by_german_verb_type, merge_tags_by_person, merge_tags_by_verb_form,
        remove_redundant_tags, sort_tags, sort_tags_by_similar,
    },
    utils::{has_kanji, is_kana, is_kanji_c},
};

pub fn postprocess_main(langs: LangSpecs, irs: &mut Tidy) {
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
    if (edition, langs.source) == (Edition::Ja, Lang::Ja) {
        let kana_to_kanji = collect_kana_to_kanji(&irs.form_map);
        postprocess_japanese_kanji_lemmas(irs, &kana_to_kanji);
        postprocess_japanese_kanji_forms(&mut irs.form_map, &kana_to_kanji);
        postprocess_japanese_odoriji_lemmas(irs);
        // Write ir message again after the changes.
        found_ir_message_impl(langs, irs);
    }

    // Comes last in case some other postprocessing logic added redundant tags.
    postprocess_forms(&mut irs.form_map);
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

fn is_kana_to_kanji_tag(tag: &str) -> bool {
    // "redirected from" are from "form-of" of wagokanji templates
    // NOTE: the redirected from is error prone since it's used by other logic
    tag == "kanji" || tag.starts_with("redirected from")
}

fn is_kana_to_kanji_form(tags: &[String]) -> bool {
    tags.iter().any(|t| is_kana_to_kanji_tag(t))
}

fn is_kana_to_kanji_pair(kana: &str, kanji: &str) -> bool {
    is_kana(kana) && has_kanji(kanji)
}

fn collect_kana_to_kanji(form_map: &FormMap) -> Map<String, Vec<String>> {
    let mut map: Map<String, Vec<String>> = Map::default();
    for (uninflected, inflected, _, _, tags) in form_map.flat_iter() {
        // NOTE: because the "redirected from" tag is error prone, be sure to only
        // add kana to kanji pairs.
        if is_kana_to_kanji_form(tags) && is_kana_to_kanji_pair(uninflected, inflected) {
            map.entry(uninflected.to_string())
                .or_default()
                .push(inflected.to_string());
        }
    }
    map
}

fn postprocess_japanese_kanji_lemmas(irs: &mut Tidy, kana_to_kanji: &Map<String, Vec<String>>) {
    let mut new_lemmas = LemmaMap::default();
    for (lemma, _, pos, info) in irs.lemma_map.flat_iter() {
        let Some(kanji_writings) = kana_to_kanji.get(lemma) else {
            continue;
        };

        for kanji in kanji_writings {
            new_lemmas.insert(kanji, lemma, pos.long(), info.clone());
        }
    }
    let n_forms_promoted = new_lemmas.len();

    for (key, infos) in new_lemmas.0 {
        let (kanji, kana_reading, pos) = key.unpack();
        // We use the wiktionary link to dedup to avoid inserting twice the same info.
        // Because the link alone includes many etymologies, we also use etymology_text
        // (unrelated to etymology, it could have been any other field), to be less coarse.
        // It is not unfallible (the proper, and expensive, solution would be to hash "info")
        // but should be valid for almost, if not every practical case.
        let mut seen = Set::default();
        for info in infos {
            if seen.insert((info.etymology_text.clone(), info.link_wiktionary.clone())) {
                debug_assert!(is_kana(kana_reading));
                irs.lemma_map.insert(kanji, kana_reading, pos.long(), info);
            }
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
        if lemmas.contains(key.uninflected.as_str()) {
            tags.retain(|tag| !is_kana_to_kanji_tag(tag))
        }
        !tags.is_empty()
    });
    let n_forms_removed = n_forms_before - irs.form_map.len();

    tracing::debug!(
        "[ja] kanji lemmas: {n_forms_promoted} forms promoted to lemmas, {n_forms_removed} forms removed"
    );
}

fn postprocess_japanese_kanji_forms(
    form_map: &mut FormMap,
    kana_to_kanji: &Map<String, Vec<String>>,
) {
    // Step 1: Collect conjugated kana forms: kana_root -> Vec<(conjugated_kana, pos, tags)>
    // uninflected=kana, inflected=conjugated_kana
    let mut kana_conjugations: Map<String, Vec<(String, String, Vec<String>)>> = Map::default();
    for (uninflected, inflected, pos, _, tags) in form_map.flat_iter() {
        if kana_to_kanji.contains_key(uninflected) {
            kana_conjugations
                .entry(uninflected.to_string())
                .or_default()
                .push((inflected.to_string(), pos.long().to_string(), tags.clone()));
        }
    }

    // Step 2: For each kanji writing, synthesize conjugated kanji forms.
    // Replace the kana root prefix in conjugated_kana with the kanji writing.
    let mut new_forms = FormMap::default();
    for (kana, kanji_writings) in kana_to_kanji {
        let Some(conjugations) = kana_conjugations.get(kana) else {
            continue;
        };
        for kanji in kanji_writings {
            for (conjugated_kana, pos, tags) in conjugations {
                if let Some(inflected_kanji) =
                    replace_kana_prefix_with_kanji(kana, kanji, conjugated_kana)
                {
                    new_forms.insert(
                        kanji,
                        &inflected_kanji,
                        pos,
                        FormSource::PostProcessed,
                        tags.clone(),
                    );
                }
            }
        }
    }

    let n_forms_synthesized = new_forms.len();
    let before = form_map.len();
    for (uninflected, inflected, pos, source, tags) in new_forms.flat_iter() {
        form_map.insert(uninflected, inflected, pos.long(), *source, tags.clone());
    }
    let n_forms_inserted = form_map.len() - before;

    tracing::debug!(
        "[ja] kanji forms: {n_forms_synthesized} synthesized, {n_forms_inserted} inserted (dedup: {})",
        n_forms_synthesized - n_forms_inserted
    );
}

/// Given:
/// - `kana_root`:       "うえかえる"
/// - `kanji_root`:      "植え換える"
/// - `conjugated_kana`: "うえかえない"
///
/// Returns `Some("植え換えない")` by finding the longest shared kana prefix,
/// then prepending the corresponding kanji prefix.
fn replace_kana_prefix_with_kanji(
    kana_root: &str,
    kanji_root: &str,
    conjugated_kana: &str,
) -> Option<String> {
    // Find the longest common prefix (by chars)
    let shared_len = kana_root
        .chars()
        .zip(conjugated_kana.chars())
        .take_while(|(a, b)| a == b)
        .count();

    if shared_len == 0 {
        return None;
    }

    let kana_suffix = &conjugated_kana[kana_root
        .char_indices()
        .nth(shared_len)
        .map_or(kana_root.len(), |(i, _)| i)..];

    // Simple heuristic: strip the non-shared kana suffix from the kanji root,
    // assuming the non-shared kana suffix corresponds to the non-shared kanji suffix.
    let kana_non_shared: String = kana_root.chars().skip(shared_len).collect();
    if kanji_root.ends_with(&kana_non_shared) {
        let kanji_prefix = &kanji_root[..kanji_root.len() - kana_non_shared.len()];
        Some(format!("{kanji_prefix}{kana_suffix}"))
    } else {
        None
    }
}

fn postprocess_japanese_odoriji_lemmas(irs: &mut Tidy) {
    let lemmas: Set<&str> = irs
        .lemma_map
        .0
        .keys()
        .map(|key| key.lemma.as_str())
        .collect();

    let mut new_lemmas = LemmaMap::default();
    for (lemma, reading, pos, info) in irs.lemma_map.flat_iter() {
        let Some(odoriji) = to_odoriji(lemma) else {
            continue;
        };
        if !lemmas.contains(odoriji.as_str()) {
            new_lemmas.insert(&odoriji, reading, pos.long(), info.clone());
        }
    }

    let n_inserted = new_lemmas.len();
    for (key, infos) in new_lemmas.0 {
        let (lemma, reading, pos) = key.unpack();
        for info in infos {
            irs.lemma_map.insert(lemma, reading, pos.long(), info);
        }
    }

    tracing::debug!("[ja] odoriji: {n_inserted} lemmas inserted");
}

/// Replaces the second kanji in a pair of identical kanji with 々.
/// e.g. 種種 -> 種々, 日日 -> 日々
/// Returns None if the word has no repeated kanji pair.
fn to_odoriji(lemma: &str) -> Option<String> {
    let chars: Vec<char> = lemma.chars().collect();
    let mut result = chars.clone();
    let mut found = false;

    for i in 0..chars.len().saturating_sub(1) {
        if is_kanji_c(chars[i]) && chars[i] == chars[i + 1] {
            result[i + 1] = '々';
            found = true;
        }
    }

    found.then(|| result.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ja_form_promotion_basic() {
        // うえかえる + うえかえない -> 植え換えない
        assert_eq!(
            replace_kana_prefix_with_kanji("うえかえる", "植え換える", "うえかえない"),
            Some("植え換えない".to_string())
        );
    }

    #[test]
    fn ja_form_promotion_full_match() {
        assert_eq!(
            replace_kana_prefix_with_kanji("たべる", "食べる", "たべる"),
            Some("食べる".to_string())
        );
    }

    #[test]
    fn ja_form_promotion_return_none() {
        // completely different kana: no shared prefix -> None
        assert_eq!(
            replace_kana_prefix_with_kanji("たべる", "食べる", "のむ"),
            None
        );
        // kanji root does not end with the non-shared kana -> None
        assert_eq!(
            replace_kana_prefix_with_kanji("いく", "行くX", "いかない"),
            None
        );
    }
}
