//! Post-processing pass.
//!
//! Deduplicates and normalizes tags across all forms.

use crate::{
    cli::LangSpecs,
    dict::main::ir::{FormMap, Tidy},
    tags::{
        merge_tags_by_case, merge_tags_by_definitiveness, merge_tags_by_gender,
        merge_tags_by_german_verb_type, merge_tags_by_person, merge_tags_by_verb_form,
        remove_redundant_tags, sort_tags, sort_tags_by_similar,
    },
};

pub fn postprocess_main(_: LangSpecs, irs: &mut Tidy) {
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
