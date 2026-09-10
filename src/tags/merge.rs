use crate::Map;

use crate::models::kaikki::Tag;
use crate::tags::TAG_SEP;

const PERSON_TAGS: [&str; 3] = ["first-person", "second-person", "third-person"];

/// Merge similar tags if the only difference is the person-tags.
///
/// F.e.
/// in:  `['first-person singular', 'third-person singular']`
/// out: `['singular first/third-person ']`
pub fn merge_tags_by_person(tags: &mut Vec<Tag>) {
    merge_tags(tags, &PERSON_TAGS, |matches| {
        // [first-person, third-person] > first/third-person
        matches
            .iter()
            // SAFETY: PERSON_TAGS contains pmatch so it always ends in -person
            .map(|pmatch| pmatch.strip_suffix("-person").unwrap())
            .collect::<Vec<_>>()
            .join(TAG_SEP)
            + "-person"
    });
}

// Uses a subset of tag_order.json cases
// TODO: At some point, generate this from that file
const CASE_TAGS: [&str; 8] = [
    "nominative",
    "genitive",
    "dative",
    "accusative",
    "vocative",
    "ablative",
    "locative",
    "partitive",
];

pub fn merge_tags_by_case(tags: &mut Vec<Tag>) {
    merge_tags_by_category(tags, &CASE_TAGS);
}

// Uses a subset of tag_order.json cases
// TODO: At some point, generate this from that file
const VERB_FORM_TAGS: [&str; 11] = [
    "imperative",
    "gerund",
    "imperfective",
    "perfective",
    "active",
    "passive",
    "participle",
    "subjunctive",
    "indicative",
    "hortative",
    "interrogative",
];

pub fn merge_tags_by_verb_form(tags: &mut Vec<Tag>) {
    merge_tags_by_category(tags, &VERB_FORM_TAGS);
}

// Uses a subset of tag_order.json cases
// TODO: At some point, generate this from that file
const DEFINITIVENESS_TAGS: [&str; 2] = ["definite", "indefinite"];

pub fn merge_tags_by_definitiveness(tags: &mut Vec<Tag>) {
    merge_tags_by_category(tags, &DEFINITIVENESS_TAGS);
}

// Uses a subset of tag_order.json cases
// TODO: At some point, generate this from that file
const GENDER_TAGS: [&str; 3] = ["masculine", "feminine", "neuter"];

pub fn merge_tags_by_gender(tags: &mut Vec<Tag>) {
    merge_tags_by_category(tags, &GENDER_TAGS);
}

// Uses a subset of tag_order.json cases
// TODO: At some point, generate this from that file
const GERMAN_VERB_TYPE_TAGS: [&str; 3] = ["weak", "strong", "mixed"];

pub fn merge_tags_by_german_verb_type(tags: &mut Vec<Tag>) {
    merge_tags_by_category(tags, &GERMAN_VERB_TYPE_TAGS);
}

/// Merge similar tags if the only difference is the category-tags.
fn merge_tags_by_category(tags: &mut Vec<Tag>, category_tags: &[&str]) {
    merge_tags(tags, category_tags, |matches| matches.join(TAG_SEP));
}

/// Generic merge function.
///
/// Note that this does not preserve logical tag order, and should be called before `sort_tag`.
fn merge_tags(tags: &mut Vec<Tag>, category_tags: &[&str], combine: impl Fn(&[&str]) -> String) {
    let contains = tags
        .iter()
        .any(|tag| tag.split(' ').any(|word| category_tags.contains(&word)));

    if !contains {
        return;
    }

    // Leave tags with same capacity since we are going to repopulate it
    let mut old_tags = Vec::with_capacity(tags.capacity());
    std::mem::swap(&mut old_tags, tags);

    let mut grouped: Map<Vec<&str>, Vec<&str>> = Map::default();

    for tag in &old_tags {
        let (matched, other_tags): (Vec<_>, Vec<_>) =
            tag.split(' ').partition(|t| category_tags.contains(t));

        match matched.as_slice() {
            [one] => grouped.entry(other_tags).or_default().push(one),
            _ => tags.push(tag.clone()),
        }
    }

    for (other_tags, mut matches) in grouped {
        matches.sort_by_key(|x| category_tags.iter().position(|p| p == x).unwrap_or(999));

        let merged = combine(&matches);

        let tag = other_tags
            .into_iter()
            .chain(std::iter::once(merged.as_str()))
            .collect::<Vec<_>>()
            .join(" ");

        tags.push(tag);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn to_string_vec(str_vec: &[&str]) -> Vec<String> {
        str_vec.iter().map(|s| (*s).to_string()).collect()
    }

    fn make_test_merge_person_tags(received: &[&str], expected: &[&str]) {
        let mut vreceived: Vec<String> = to_string_vec(received);
        let vexpected: Vec<String> = to_string_vec(expected);
        merge_tags_by_person(&mut vreceived);
        assert_eq!(vreceived, vexpected);
    }

    #[test]
    fn merge_person_tags1() {
        make_test_merge_person_tags(
            &[
                "first-person singular present",
                "third-person singular present",
            ],
            &["singular present first/third-person"],
        );
    }

    // Improvement over the original that would return:
    // "first/second-person singular past",
    // "third-person singular past",
    #[test]
    fn merge_person_tags2() {
        make_test_merge_person_tags(
            &[
                "first-person singular past",
                "second-person singular past",
                "third-person singular past",
            ],
            &["singular past first/second/third-person"],
        );
    }
}
