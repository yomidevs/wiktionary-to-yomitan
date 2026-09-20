use crate::Map;

use crate::models::kaikki::Tag;
use crate::tags::TAG_SEP;

/// Define the merge categories with their tag words.
///
/// Registering the words once via the macro gives both the list the merges use, and the 
/// [`tag_category`] match, so the two cannot drift apart.
macro_rules! categories {
    ($($variant:ident => [$($word:literal),+ $(,)?],)+) => {
        /// The categories we merge tags by.
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum Category {
            $($variant,)+
        }

        impl Category {
            /// In merge order.
            const ALL: [Self; [$(Self::$variant,)+].len()] = [$(Self::$variant,)+];

            /// Its tag words, in the order they are merged in.
            const fn tags(self) -> &'static [&'static str] {
                match self {
                    $(Self::$variant => &[$($word,)+],)+
                }
            }
        }

        /// The category of a tag word, if any.
        fn tag_category(word: &str) -> Option<Category> {
            match word {
                $($($word)|+ => Some(Category::$variant),)+
                _ => None,
            }
        }
    };
}

// Everything but the person tags is a subset of tag_order.json.
// TODO: At some point, generate those from that file
categories! {
    Person => ["first-person", "second-person", "third-person"],
    Case => [
        "nominative",
        "genitive",
        "dative",
        "accusative",
        "vocative",
        "ablative",
        "locative",
        "partitive",
    ],
    VerbForm => [
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
    ],
    // [ko-en]
    Definitiveness => ["definite", "indefinite"],
    Gender => ["masculine", "feminine", "neuter"],
    GermanVerbType => ["weak", "strong", "mixed"],
}

impl Category {
    /// Merge tags that only differ in the words of this category.
    fn merge(self, tags: &mut Vec<Tag>) {
        match self {
            // first-person + third-person > first/third-person
            Self::Person => merge_tags(tags, self, |matches| {
                matches
                    .iter()
                    // SAFETY: the person tags always end in -person
                    .map(|pmatch| pmatch.strip_suffix("-person").unwrap())
                    .collect::<Vec<_>>()
                    .join(TAG_SEP)
                    + "-person"
            }),
            _ => merge_tags(tags, self, |matches| matches.join(TAG_SEP)),
        }
    }
}

/// Apply every category merge.
///
/// Most forms have no category word at all, so classify the words once and skip the merges that
/// cannot match. Note that some merges are only relevant for certain editions, but they are
/// cheap enough not to gate on (Edition, Lang) pairs.
pub fn merge_tags_by_categories(tags: &mut Vec<Tag>) {
    let mut present = [false; Category::ALL.len()];
    for tag in &*tags {
        for word in tag.split(' ') {
            if let Some(category) = tag_category(word) {
                present[category as usize] = true;
            }
        }
    }

    for category in Category::ALL {
        if present[category as usize] {
            category.merge(tags);
        }
    }
}

/// Generic merge function.
///
/// Note that this does not preserve logical tag order, and should be called before `sort_tag`.
fn merge_tags(tags: &mut Vec<Tag>, category: Category, combine: impl Fn(&[&str]) -> String) {
    let category_tags = category.tags();

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
        Category::Person.merge(&mut vreceived);
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
