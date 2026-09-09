mod tags_constants;
pub use tags_constants::Pos;
use tags_constants::{TAG_BANK, TAG_ORDER};

mod tags_localization;
pub use tags_localization::*;

mod merge;
pub use merge::*;

use std::cmp::Ordering;
use std::sync::LazyLock;

use crate::Map;
use crate::lang::Lang;
use crate::models::kaikki::Tag;
use crate::models::yomitan::TagInfo;

/// Tag separator.
///
/// Needs to be synced between merge logic and sorting.
const TAG_SEP: &str = "/";

/// Tags that are blacklisted if they happen at *some* expanded form @ tidy
pub const BLACKLISTED_FORM_TAGS: [&str; 14] = [
    "inflection-template",
    "table-tags",
    "canonical",
    "class",
    "error-unknown-tag",
    "error-unrecognized-form",
    "includes-article",
    "obsolete",
    "archaic",
    "used-in-the-form",
    "romanization",
    "dated",
    "auxiliary",
    // multiword-construction was in REDUNDANT_TAGS in the original.
    // Yet it only seems to give noise for the fr-en edition (@ prendre):
    // * Form: 'present indicative of avoir + past participle' ???
    // * Tags: ["indicative", "multiword-construction", "perfect", "present"]
    //
    // It also removes valid german forms that are nonetheless most useless:
    // * werde gepflogen haben (for pflegen)
    // (note that gepflogen is already added)
    // This was considered ok. To revisit if it is more intrusive in other languages.
    "multiword-construction",
];
/// Tags that are blacklisted if they happen at *every* expanded form @ tidy
pub const IDENTITY_FORM_TAGS: [&str; 3] = ["nominative", "singular", "infinitive"];
/// Tags that we just remove from forms @ tidy
pub const REDUNDANT_FORM_TAGS: [&str; 1] = ["combined-form"];

/// Get the substring before `TAG_SEP`, or default to the whole string.
fn before_sep(s: &str) -> &str {
    s.split_once(TAG_SEP).map_or(s, |(before, _)| before)
}

// TODO: instead of doing TAG_ORDER.index, rewrite build.py to produce a match
// statement that returns directly the order.
//
/// Sort tags by their position in the `tag_order.json` file.
///
/// If a tag contains `TAG_SEP`, the substring before `TAG_SEP` is used instead.
/// This is done so we can sort merged tags: nominative/accusative etc.
///
/// Expects (but does not check) tags WITHOUT spaces.
pub fn sort_tags(tags: &mut [&str]) {
    debug_assert!(tags.iter().all(|tag| !tag.contains(' ')));

    tags.sort_by(|a, b| {
        let bef_a = before_sep(a);
        let bef_b = before_sep(b);

        let index_a = TAG_ORDER.iter().position(|&x| x == bef_a);
        let index_b = TAG_ORDER.iter().position(|&x| x == bef_b);

        match (index_a, index_b) {
            (Some(i), Some(j)) => i.cmp(&j),   // both found → compare positions
            (Some(_), None) => Ordering::Less, // found beats not-found
            (None, Some(_)) => Ordering::Greater,
            // This seems better but it's different from the original
            // (None, None) => a.cmp(b),        // neither found → alphabetical fallback
            (None, None) => Ordering::Equal, // neither found → do nothing
        }
    });
}

/// Sort tags by word-by-word lexicographical similarity, grouping tags that
/// share the same leading words (shorter prefix-only tags sort first).
///
/// Expects (but does not check) tags WITH spaces.
pub fn sort_tags_by_similar(tags: &mut [Tag]) {
    tags.sort_by(|a, b| {
        let mut a_iter = a.split(' ');
        let mut b_iter = b.split(' ');

        loop {
            match (a_iter.next(), b_iter.next()) {
                (Some(a_word), Some(b_word)) => match a_word.cmp(b_word) {
                    Ordering::Equal => continue,
                    non_eq => return non_eq,
                },
                (Some(_), None) => return Ordering::Greater,
                (None, Some(_)) => return Ordering::Less,
                (None, None) => return Ordering::Equal,
            }
        }
    });
}

/// Remove tag1 if there is a tag2 such that tag1 <= tag2.
///
/// Expects (but does not check) tags WITH spaces.
///
/// Examples:
/// * ["a b", "b a"] -> ["b a"]
/// * ["s no ne", "ne s no", "ne", "no"] -> ["ne s no"]
pub fn remove_redundant_tags(tags: &mut Vec<Tag>) {
    let mut keep = vec![true; tags.len()];

    for i in 0..tags.len() {
        if !keep[i] {
            continue;
        }
        for j in (i + 1)..tags.len() {
            if !keep[j] {
                continue;
            }

            if tags_are_subset(&tags[i], &tags[j]) {
                keep[i] = false;
                break;
            } else if tags_are_subset(&tags[j], &tags[i]) {
                keep[j] = false;
            }
        }
    }

    let mut idx = 0;
    tags.retain(|_| {
        let k = keep[idx];
        idx += 1;
        k
    });
}

/// Check if all words in `a` are present in `b`, f.e. "foo bar" is subset of "bar foo baz".
fn tags_are_subset(a: &str, b: &str) -> bool {
    a.split(' ')
        .all(|a_word| b.split(' ').any(|b_word| b_word == a_word))
}

// Note that while target is an Edition for the main dictionary, it can be any Lang
// for the glossary dictionary, which also uses tags.
//
/// Return a Vec<[`TagInfo`]> from [`TAG_BANK`] (`tag_bank_terms.json`).
pub fn get_tag_bank_as_tag_info(target: Lang) -> Vec<TagInfo> {
    if has_locale(target) {
        TAG_BANK
            .iter()
            .map(
                |&(short_tag, category, sort_order, long_tag_aliases, popularity_score)| {
                    let (short_tag_loc, long_tag_loc) = match localize_tag(target, short_tag) {
                        Some((short, long)) => (short, long),
                        // This short_tag hasn't been localized yet, use the English version
                        None => (short_tag, long_tag_aliases[0]),
                    };

                    TagInfo {
                        short_tag: short_tag_loc.to_string(),
                        category: category.to_string(),
                        sort_order,
                        long_tag: long_tag_loc.to_string(),
                        popularity_score,
                    }
                },
            )
            .collect()
    } else {
        TAG_BANK.iter().map(TagInfo::new).collect()
    }
}

/// Find the (long) tag in [`TAG_BANK`] (`tag_bank_terms.json`) and return its [`TagInfo`] if any.
pub fn find_tag_in_bank(tag: &str) -> Option<TagInfo> {
    // Index the aliases once, so the search is O(1) instead of O(n).
    static ALIAS_INDEX: LazyLock<Map<&'static str, usize>> = LazyLock::new(|| {
        let mut index = Map::default();
        for (position, entry) in TAG_BANK.iter().enumerate() {
            for alias in entry.3 {
                index.entry(*alias).or_insert(position);
            }
        }
        index
    });

    ALIAS_INDEX
        .get(tag)
        .map(|position| TagInfo::new(&TAG_BANK[*position]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn to_string_vec(str_vec: &[&str]) -> Vec<String> {
        str_vec.iter().map(|s| (*s).to_string()).collect()
    }

    fn to_str_vec<'a>(str_vec: &[&'a str]) -> Vec<&'a str> {
        str_vec.to_vec()
    }

    // This imitates the original. Can be removed if sort_tags logic changes.
    #[test]
    fn sort_tags_not_found() {
        let tag_not_found = "__sentinel";
        assert!(!TAG_ORDER.contains(&tag_not_found));
        let mut received = to_str_vec(&[tag_not_found, "Gheg"]);
        let expected = to_string_vec(&[tag_not_found, "Gheg"]);
        sort_tags(&mut received);
        assert_eq!(received, expected);
    }

    #[test]
    fn sort_tags_base() {
        let tag1 = "genitive";
        let tag2 = "accusative";
        let index_1 = TAG_ORDER.iter().position(|&x| x == tag1).unwrap();
        let index_2 = TAG_ORDER.iter().position(|&x| x == tag2).unwrap();
        assert!(index_1 < index_2);

        let mut received = to_str_vec(&[tag2, tag1]);
        let expected = to_string_vec(&[tag1, tag2]);
        sort_tags(&mut received);
        assert_eq!(received, expected);
    }

    #[test]
    fn sort_tags_with_separator() {
        let tag1 = "genitive/dative";
        let tag2 = "accusative";

        let mut received = to_str_vec(&[tag2, tag1]);
        let expected = to_string_vec(&[tag1, tag2]);
        sort_tags(&mut received);
        assert_eq!(received, expected);
    }

    fn make_test_sort_tags_by_similar(received: &[&str], expected: &[&str]) {
        let mut vreceived: Vec<String> = to_string_vec(received);
        let vexpected: Vec<String> = to_string_vec(expected);
        sort_tags_by_similar(&mut vreceived);
        assert_eq!(vreceived, vexpected);
    }

    #[test]
    fn sort_tags_by_similar1() {
        make_test_sort_tags_by_similar(&["singular", "accusative"], &["accusative", "singular"]);
    }

    #[test]
    fn sort_tags_by_similar2() {
        make_test_sort_tags_by_similar(
            &["accusative", "singular", "neuter", "nominative", "vocative"],
            &["accusative", "neuter", "nominative", "singular", "vocative"],
        );
    }

    #[test]
    fn sort_tags_by_similar3() {
        make_test_sort_tags_by_similar(
            &["dual nominative", "accusative dual", "dual vocative"],
            &["accusative dual", "dual nominative", "dual vocative"],
        );
    }

    #[test]
    fn remove_redundant_tags1() {
        let mut received = to_string_vec(&["foo", "bar", "foo bar", "foo bar zee"]);
        let expected = to_string_vec(&["foo bar zee"]);
        remove_redundant_tags(&mut received);
        assert_eq!(received, expected);
    }

    #[test]
    fn remove_redundant_tags2() {
        let mut received = to_string_vec(&[
            "first-person singular indicative preterite",
            "first-person singular preterite",
        ]);
        let expected = to_string_vec(&["first-person singular indicative preterite"]);
        remove_redundant_tags(&mut received);
        assert_eq!(received, expected);
    }

    #[test]
    fn remove_redundant_tags_duplicates1() {
        let mut received = to_string_vec(&["a b", "a b"]);
        let expected = to_string_vec(&["a b"]);
        remove_redundant_tags(&mut received);
        assert_eq!(received, expected);
    }

    #[test]
    fn remove_redundant_tags_duplicates2() {
        let mut received = to_string_vec(&["a b", "b a"]);
        let expected = to_string_vec(&["b a"]);
        remove_redundant_tags(&mut received);
        assert_eq!(received, expected);
    }

    #[test]
    fn remove_redundant_tags_duplicates3() {
        let mut received = to_string_vec(&["a b", "c a b", "b a", "b a c", "c b a"]);
        let expected = to_string_vec(&["c b a"]);
        remove_redundant_tags(&mut received);
        assert_eq!(received, expected);
    }

    #[test]
    fn remove_redundant_tags_duplicates4() {
        let mut received = to_string_vec(&["s no ne", "ne s no", "ne", "no"]);
        let expected = to_string_vec(&["ne s no"]);
        remove_redundant_tags(&mut received);
        assert_eq!(received, expected);
    }

    #[test]
    fn tags_subsets() {
        assert!(tags_are_subset("foo bar", "bar foo baz"));
        assert!(!tags_are_subset("foo qux", "foo bar baz"));
    }

    use crate::{lang::Lang, models::yomitan::TagInfo};

    #[test]
    fn locale_ja_tag_bank() {
        let tag_bank = get_tag_bank_as_tag_info(Lang::Ja);
        let entry = ("動", "partOfSpeech", -2, &["動詞"][..], 2);
        let loc_tag_info = TagInfo::new(&entry);
        assert!(tag_bank.contains(&loc_tag_info));
    }

    #[test]
    fn locale_ja_translate_tag() {
        let loc_tag = localize_tag(Lang::Ja, "v");
        assert_eq!(loc_tag, Some(("動", "動詞")));
    }
}
