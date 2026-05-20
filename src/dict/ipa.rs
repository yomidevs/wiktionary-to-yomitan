//! Ipa and IpaMerged dictionaries.

use crate::{
    Map,
    cli::{IpaArgs, IpaMergedArgs, LangSpecs},
    dict::{Dictionary, Langs, main::get_reading},
    lang::{Edition, Lang},
    models::{
        kaikki::WordEntry,
        yomitan::{
            Ipa, PhoneticTranscription, TermMetaBankEntry, TermPhoneticTranscription, YomitanDict,
        },
    },
    tags::{find_tag_in_bank, localize_tag},
};

#[derive(Debug, Clone, Copy)]
pub struct DIpa;

#[derive(Debug, Clone, Copy)]
pub struct DIpaMerged;

impl Dictionary for DIpa {
    type A = IpaArgs;
    type I = IIpa;

    fn process(&self, langs: Langs, entry: &WordEntry, irs: &mut Self::I) {
        process_ipa(langs.edition, langs.source, langs.target, entry, irs);
    }

    fn to_yomitan(&self, _: LangSpecs, irs: &Self::I) -> YomitanDict {
        YomitanDict::new(vec![], vec![], to_yomitan_ipa(irs))
    }
}

impl Dictionary for DIpaMerged {
    type A = IpaMergedArgs;
    type I = IIpa;

    fn process(&self, langs: Langs, entry: &WordEntry, irs: &mut Self::I) {
        process_ipa(langs.edition, langs.source, langs.target, entry, irs);
    }

    fn postprocess(&self, irs: &mut Self::I) {
        // Sorting is not needed ~ just for visibility
        irs.sort_unstable_keys();
    }

    fn to_yomitan(&self, _: LangSpecs, irs: &Self::I) -> YomitanDict {
        YomitanDict::new(vec![], vec![], to_yomitan_ipa(irs))
    }
}

/// Normalize IPA notation: convert `\X\`, bare `X` to `/X/`, preserve `[X]`
///
/// `\X\` is used in the French edition for some reason, and `X` is just laziness.
///
/// ## Difference between `/X/` and `[X]`
///
/// - `/ ... /` is for phonemic transcription
/// - `[ ... ]` is for phonetic transcription
///
/// For example, the English words pin and spin become:
/// - `/pɪn/` and `/spɪn/`
/// - `[pʰɪn]` and `[spɪn]`.
///
/// See <https://en.wiktionary.org/wiki/Wiktionary:International_Phonetic_Alphabet>
fn normalize_ipa(text: &str) -> String {
    let fst = text.chars().next();
    let lst = text.chars().last();

    match (fst, lst) {
        (Some('['), Some(']')) => text.to_string(),
        (Some('/'), Some('/')) => text.to_string(),
        (Some('\\'), Some('\\')) if text.len() > 1 => format!("/{}/", &text[1..text.len() - 1]),
        _ => format!("/{text}/"),
    }
}

/// Extract inner IPA content without delimiters for comparison
fn ipa_inner(text: &str) -> &str {
    let fst = text.chars().next();
    let lst = text.chars().last();

    match (fst, lst) {
        (Some('['), Some(']')) => &text[1..text.len() - 1],
        (Some('/'), Some('/')) | (Some('\\'), Some('\\')) if text.len() > 1 => {
            &text[1..text.len() - 1]
        }
        _ => text,
    }
}

fn is_phonetic(text: &str) -> bool {
    text.starts_with('[') && text.ends_with(']')
}

// Grouping by ipa is done at process_ipa
fn get_ipas(entry: &WordEntry) -> Vec<Ipa> {
    entry
        .sounds
        .iter()
        .filter_map(|sound| {
            if sound.ipa.is_empty() {
                return None;
            }
            let mut tags = sound.tags.clone();
            if !sound.note.is_empty() {
                tags.push(sound.note.clone());
            }
            Some(Ipa {
                ipa: normalize_ipa(&sound.ipa),
                tags,
            })
        })
        .collect()
}

/// ((lemma, reading), transcription)
type IIpa = Map<(String, String), Vec<Ipa>>;

fn process_ipa(edition: Edition, source: Lang, target: Lang, entry: &WordEntry, irs: &mut IIpa) {
    let mut ipas = get_ipas(entry);

    if ipas.is_empty() {
        return;
    }

    // This replacing with the short tag will still show the long version on hover,
    // even though they are not really top-level tags (in the sense of the main dict)
    for ipa in &mut ipas {
        for tag in &mut ipa.tags {
            // tracing::warn!("tag {tag} @ {} (ed: {edition})", &entry.word);
            if let Some(tag_info) = find_tag_in_bank(tag) {
                // tracing::warn!("found tag {tag_info:?}");
                *tag = match localize_tag(target, &tag_info.short_tag) {
                    Some((short, _)) => {
                        // tracing::warn!("localized short tag {short:?}");
                        short.to_string()
                    }
                    None => (*tag_info.short_tag).to_string(),
                }
            }
        }
    }

    let reading = get_reading(edition, source, entry).unwrap_or_else(|| entry.word.clone());
    let existing = irs.entry((entry.word.clone(), reading)).or_default();
    for ipa in ipas {
        let inner = ipa_inner(&ipa.ipa);
        if let Some(existing_ipa) = existing.iter_mut().find(|e| ipa_inner(&e.ipa) == inner) {
            // Prefer phonetic [X] over phonemic /X/ (more specific)
            if is_phonetic(&ipa.ipa) {
                existing_ipa.ipa = ipa.ipa;
            }
            for tag in ipa.tags {
                if !existing_ipa.tags.contains(&tag) {
                    existing_ipa.tags.push(tag);
                }
            }
        } else {
            existing.push(ipa);
        }
    }
}

fn to_yomitan_ipa(irs: &IIpa) -> Vec<TermMetaBankEntry> {
    irs.into_iter()
        .map(|((lemma, reading), transcriptions)| {
            // NOTE: sorting is tricky because the order in Wiktionary may matter, with the first
            // result being the most relevant (not always, remains to be tested).
            // This is relevant for X-Y-ipa dicts, for merged X-ipa dicts the order is completely
            // random, as in it depends on edition iteration order, and therefore sorting is much
            // more justified.
            //
            // transcriptions.sort_unstable_by(|a, b| ipa_inner(&a.ipa).cmp(ipa_inner(&b.ipa)));

            TermMetaBankEntry::TermPhoneticTranscription(TermPhoneticTranscription::new(
                lemma.clone(),
                PhoneticTranscription {
                    reading: reading.clone(),
                    transcriptions: transcriptions.clone(),
                },
            ))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::models::kaikki::Sound;

    impl Sound {
        fn new(ipa: &str) -> Self {
            Self {
                ipa: ipa.into(),
                ..Default::default()
            }
        }

        fn with_tag(ipa: &str, tag: &str) -> Self {
            Self {
                ipa: ipa.into(),
                tags: vec![tag.into()],
                ..Default::default()
            }
        }
    }

    #[test]
    fn process_ipa_merged_basic() {
        let dict = DIpaMerged;
        let langs = Langs::new(Edition::En, Lang::Grc, Lang::Sh);
        let mut entry = WordEntry::default();
        entry.sounds = vec![
            Sound::new("/ipa1/"), // '/' phonemic gets replaced by '[]' phonetic...
            Sound::new("[ipa1]"),
            Sound::new("/ipa1/"), // ...independently of order
            Sound::new("[ipa2]"),
        ];

        let mut irs = IIpa::default();
        dict.process(langs, &entry, &mut irs);

        assert_eq!(irs.len(), 1);

        let transcriptions = irs.values().next().unwrap();
        assert_eq!(transcriptions.len(), 2);
        assert_eq!(&transcriptions[0].ipa, "[ipa1]");
        assert_eq!(&transcriptions[1].ipa, "[ipa2]");
    }

    #[test]
    fn process_ipa_tag() {
        let dict = DIpa;
        let langs = Langs::new(Edition::En, Lang::La, Lang::La); // irrelevant
        let mut entry = WordEntry::default();
        entry.sounds = vec![
            Sound::with_tag("[ipa1]", "tag1"),
            Sound::with_tag("[ipa2]", "modern Italianate Ecclesiastical"),
        ];

        let mut irs = IIpa::default();
        dict.process(langs, &entry, &mut irs);

        assert_eq!(irs.len(), 1);

        let transcriptions = irs.values().next().unwrap();
        assert_eq!(transcriptions.len(), 2);

        assert_eq!(&transcriptions[0].ipa, "[ipa1]");
        assert_eq!(&transcriptions[1].ipa, "[ipa2]");

        // Check that tags are properly simplified
        assert_eq!(&transcriptions[0].tags[0], "tag1");
        assert_eq!(&transcriptions[1].tags[0], "⛪");
    }

    #[test]
    fn process_ipa_merged_tag_merge() {
        let dict = DIpaMerged;
        let (source, target) = (Lang::La, Lang::La);

        let edition = Edition::En;
        let langs = Langs::new(edition, source, target);
        let mut entry = WordEntry::default();
        entry.sounds = vec![Sound::with_tag("ipa1", "tag1")];

        let mut irs = IIpa::default();
        dict.process(langs, &entry, &mut irs);

        let edition = Edition::De;
        let langs = Langs::new(edition, source, target);
        let mut entry = WordEntry::default();
        // same ipa, different tag (coming from another edition)
        entry.sounds = vec![Sound::with_tag("ipa1", "tag2")];
        dict.process(langs, &entry, &mut irs);

        let transcriptions = irs.values().next().unwrap();
        assert_eq!(transcriptions.len(), 1);
        // Both tags should be present after merging
        assert!(transcriptions[0].tags.contains(&"tag1".to_string()));
        assert!(transcriptions[0].tags.contains(&"tag2".to_string()));
    }

    #[test]
    fn process_ipa_merged_postprocess_order() {
        let dict = DIpaMerged;
        let (source, target) = (Lang::La, Lang::La);
        let langs = Langs::new(Edition::En, source, target);

        let mut irs = IIpa::default();

        let mut entry = WordEntry::default();
        entry.word = "zebra".to_string();
        entry.sounds = vec![Sound::new("ipa1")];
        dict.process(langs, &entry, &mut irs);

        let mut entry = WordEntry::default();
        entry.word = "apple".to_string();
        entry.sounds = vec![Sound::new("ipa2")];
        dict.process(langs, &entry, &mut irs);

        dict.postprocess(&mut irs);

        let keys: Vec<&String> = irs.keys().map(|(word, _)| word).collect();
        assert_eq!(keys, vec!["apple", "zebra"]);
    }
}
