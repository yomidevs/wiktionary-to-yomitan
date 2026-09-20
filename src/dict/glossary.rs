//! `Glossary` and `GlossaryExtended` dictionaries.

use crate::{
    Map, Set,
    cli::{GlossaryArgs, GlossaryExtendedArgs, LangSpecs},
    dict::{Dictionary, Langs, main::get_reading, rules::rule_identifiers},
    lang::{Edition, Lang},
    models::{
        kaikki::{Translation, WordEntry},
        yomitan::{DetailedDefinition, NTag, Node, TermBankEntry, YomitanDict, wrap},
    },
    tags::{Pos, find_tag_in_bank, localize_tag_info},
};

#[derive(Debug, Clone, Copy)]
pub struct DGlossary;

#[derive(Debug, Clone, Copy)]
pub struct DGlossaryExtended;

impl Dictionary for DGlossary {
    type A = GlossaryArgs;
    type I = Vec<TermBankEntry>;

    fn process(&self, langs: Langs, entry: &WordEntry, irs: &mut Self::I) {
        process_glossary(langs.edition, langs.target, entry, irs);
    }

    fn to_yomitan(&self, _: LangSpecs, irs: &Self::I) -> YomitanDict {
        YomitanDict::new(irs.clone(), vec![], vec![])
    }
}

fn process_glossary(
    source: Edition,
    target: Lang,
    entry: &WordEntry,
    irs: &mut Vec<TermBankEntry>,
) {
    let mut translations: Map<&str, Vec<String>> = Map::default();
    for translation in entry.non_trivial_translations() {
        if translation.lang_code == target.iso() {
            translations
                .entry(&translation.sense)
                .or_default()
                .push(translation.word.clone());
        }
    }

    if translations.is_empty() {
        return;
    }

    let mut definitions = Vec::new();
    for (sense, translations) in translations {
        if sense.is_empty() {
            definitions.extend(translations.into_iter().map(DetailedDefinition::Text));
            continue;
        }

        definitions.push(DetailedDefinition::structured(wrap(
            NTag::Div,
            "",
            Node::Array(vec![
                wrap(NTag::Div, "sense-label", Node::Text(sense.to_string())),
                wrap(
                    NTag::Ul,
                    // We add this "data-sc-content=glossary" for compact mode
                    // https://github.com/yomidevs/yomitan/blob/master/ext/css/structured-content.css#L240
                    "glossary",
                    Node::Array(
                        translations
                            .into_iter()
                            .map(|translation| wrap(NTag::Li, "", Node::Text(translation)))
                            .collect(),
                    ),
                ),
            ]),
        )));
    }

    let reading = match get_reading(source, source.into(), entry) {
        Some(reading) if reading != entry.word => reading,
        _ => String::new(),
    };
    let definition_tags = match find_tag_in_bank(&entry.pos) {
        Some(mut tag_info) => {
            localize_tag_info(target, &mut tag_info);
            vec![tag_info]
        }
        None => vec![],
    };
    let pos = Pos::from(entry.pos.as_str());
    let rules = rule_identifiers(source.into(), &entry.word, &[pos.short().to_string()]);

    irs.push(TermBankEntry::new(
        entry.word.clone(),
        reading,
        definition_tags,
        rules,
        definitions,
    ));
}

/// (sense, translations). The sense is in the edition's language: it only groups, and is
/// never rendered.
type Senses = Vec<(String, Vec<String>)>;

/// (lemma, reading, pos, senses). The pos is the pivot entry's, not the lemma's.
type IGlossaryExtended = Vec<(String, String, Pos, Senses)>;

/// Merge target of [`DGlossaryExtended::postprocess`]:
/// `(lemma, reading, pos) -> sense -> translations`.
type MergedSenses = Map<(String, String, Pos), Map<String, Set<String>>>;

impl Dictionary for DGlossaryExtended {
    type A = GlossaryExtendedArgs;
    type I = IGlossaryExtended;

    fn supports_probe(&self) -> bool {
        false
    }

    fn process(&self, langs: Langs, entry: &WordEntry, irs: &mut Self::I) {
        process_glossary_extended(langs.source, langs.target, entry, irs);
    }

    // TODO: change type "I" to not have to merge lemmas here
    fn postprocess(&self, _: LangSpecs, irs: &mut Self::I) {
        let readings = single_readings(irs);
        let mut map = MergedSenses::default();

        for (lemma, mut reading, pos, senses) in irs.drain(..) {
            if reading.is_empty() {
                reading = readings
                    .get(&(lemma.clone(), pos))
                    .cloned()
                    .unwrap_or_default();
            }

            let merged = map.entry((lemma, reading, pos)).or_default();

            for (sense, translations) in senses {
                merged.entry(sense).or_default().extend(translations);
            }
        }

        irs.extend(map.into_iter().map(|((lemma, reading, pos), senses)| {
            let senses = senses
                .into_iter()
                .map(|(sense, translations)| (sense, translations.into_iter().collect()))
                .collect();
            (lemma, reading, pos, senses)
        }));
    }

    fn to_yomitan(&self, langs: LangSpecs, irs: &Self::I) -> YomitanDict {
        YomitanDict::new(
            to_yomitan_glossary_extended(langs.source, langs.target, irs),
            vec![],
            vec![],
        )
    }
}

fn process_glossary_extended(
    source: Lang,
    target: Lang,
    entry: &WordEntry,
    irs: &mut IGlossaryExtended,
) {
    let mut translations: Map<&str, (Vec<&str>, Vec<&Translation>)> = Map::default();

    for translation in entry.non_trivial_translations() {
        if translation.lang_code == target.iso() {
            translations
                .entry(&translation.sense)
                .or_default()
                .0
                .push(&translation.word);
        }

        if translation.lang_code == source.iso() {
            translations
                .entry(&translation.sense)
                .or_default()
                .1
                .push(translation);
        }
    }

    // We only keep translations with matches in both languages (source and target)
    translations.retain(|_, (targets, sources)| !targets.is_empty() && !sources.is_empty());

    if translations.is_empty() {
        return;
    }

    // A "semi" cartesian product. See the test below.
    irs.extend(translations.iter().flat_map(|(sense, (targets, sources))| {
        sources.iter().map(|translation| {
            (
                translation.word.clone(),
                translation_reading(source, translation),
                Pos::from(entry.pos.as_str()),
                vec![(
                    (*sense).to_string(),
                    targets.iter().map(|def| (*def).to_string()).collect(),
                )],
            )
        })
    }));
}

/// Reading for a translated word, empty when there is none.
fn translation_reading(source: Lang, translation: &Translation) -> String {
    let reading = match source {
        Lang::Ja => &translation.alt,
        Lang::Zh | Lang::Fa => &translation.roman,
        _ => return String::new(),
    };

    if *reading == translation.word {
        return String::new();
    }

    reading.clone()
}

/// The reading of every word that has exactly one, keyed by `(lemma, pos)`.
///
/// A translation table does not repeat the reading on every row, so the same word arrives
/// both with and without one from Kaikki.
fn single_readings(irs: &IGlossaryExtended) -> Map<(String, Pos), String> {
    let mut readings: Map<(&str, Pos), Option<&str>> = Map::default();

    for (lemma, reading, pos, _) in irs {
        if reading.is_empty() {
            continue;
        }

        let known = readings
            .entry((lemma.as_str(), *pos))
            .or_insert(Some(reading.as_str()));
        if *known != Some(reading.as_str()) {
            *known = None;
        }
    }

    readings
        .into_iter()
        .filter_map(|((lemma, pos), reading)| {
            Some(((lemma.to_string(), pos), reading?.to_string()))
        })
        .collect()
}

fn to_yomitan_glossary_extended(
    source: Lang,
    target: Lang,
    irs: &IGlossaryExtended,
) -> Vec<TermBankEntry> {
    irs.iter()
        .map(|(lemma, reading, pos, senses)| {
            let definition_tags = match find_tag_in_bank(pos.long()) {
                Some(mut tag_info) => {
                    localize_tag_info(target, &mut tag_info);
                    vec![tag_info]
                }
                None => vec![],
            };
            let rules = rule_identifiers(source, lemma, &[pos.short().to_string()]);

            // One definition per sense. The label is only a grouping key: it is written in
            // the edition's language, which is neither the source nor the target. Senses
            // that share a translation set collapse into a single definition.
            let mut seen = Set::default();
            let definitions = senses
                .iter()
                .filter(|(_, translations)| {
                    let mut key: Vec<&str> = translations.iter().map(String::as_str).collect();
                    key.sort_unstable();
                    seen.insert(key)
                })
                .map(|(_, translations)| DetailedDefinition::Text(translations.join(", ")))
                .collect();

            TermBankEntry::new(
                lemma.clone(),
                reading.clone(),
                definition_tags,
                rules,
                definitions,
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::models::kaikki::Translation;

    impl Translation {
        fn new(lang_code: &str, sense: &str, word: &str) -> Self {
            Self {
                lang_code: lang_code.into(),
                sense: sense.into(),
                word: word.into(),
                ..Default::default()
            }
        }
    }

    // cf. https://en.wiktionary.org/wiki/Gibraltar
    // {
    //     English (sense):       "British overseas territory"
    //     Albanian (sh):         ["Gjibraltar", "Gjibraltari"]
    //     Greek, Ancient (grc):  ["Ἡράκλειαι στῆλαι", "Κάλπη"]
    // }
    //
    //     source                            target (what we search)
    // >>> ["Gjibraltar", "Gjibraltari"]  <> "Ἡράκλειαι στῆλαι"
    // >>> ["Gjibraltar", "Gjibraltari"]  <> "Κάλπη"
    #[test]
    fn process_glossary_extended_basic() {
        let dict = DGlossaryExtended;
        let langs = Langs::new(Edition::En, Lang::Grc, Lang::Sh);
        let mut entry = WordEntry::default();
        entry.pos = "noun".to_string();
        entry.translations = vec![
            Translation::new("grc", "British overseas territory", "Ἡράκλειαι στῆλαι"),
            Translation::new("grc", "British overseas territory", "Ἡράκλειαι στῆλαι"),
            Translation::new("grc", "British overseas territory", "Κάλπη"),
            Translation::new("sh", "British overseas territory", "Gibraltar"),
            Translation::new("sh", "British overseas territory", "Gjibraltari"),
            Translation::new("sh", "Different sense", "Foo"),
        ];

        let mut irs = Vec::new();
        dict.process(langs, &entry, &mut irs);

        // Empty translations should not change anything
        let entry = WordEntry::default();
        dict.process(langs, &entry, &mut irs);

        assert_eq!(irs.len(), 3);

        let (lemma1, _, pos, senses1) = &irs[0];
        let (lemma2, _, _, senses2) = &irs[1];
        let (lemma3, _, _, senses3) = &irs[2];

        assert_eq!(pos.long(), "noun");
        assert_eq!(lemma1, "Ἡράκλειαι στῆλαι");
        assert_eq!(lemma2, "Ἡράκλειαι στῆλαι");
        assert_eq!(lemma3, "Κάλπη");

        let expected = vec![(
            "British overseas territory".to_string(),
            vec!["Gibraltar".to_string(), "Gjibraltari".to_string()],
        )];
        assert_eq!(senses1, &expected);
        assert_eq!(senses2, &expected);
        assert_eq!(senses3, &expected);

        dict.postprocess(LangSpecs::from(langs), &mut irs);
        assert_eq!(irs.len(), 2);

        let yomitan_entries = to_yomitan_glossary_extended(langs.source, langs.target, &irs);
        assert_eq!(yomitan_entries.len(), 2);
        let term_bank = yomitan_entries.first().unwrap();
        assert_eq!(term_bank.definition_tags[0].short_tag, "n");
    }

    /// Each sense of a pivot entry becomes its own definition, and senses that share a
    /// translation set collapse into one.
    #[test]
    fn process_glossary_extended_groups_by_sense() {
        let dict = DGlossaryExtended;
        let langs = Langs::new(Edition::En, Lang::Es, Lang::De);

        // es "banco" is the translation of three senses of "bank", two of which share the
        // same German word.
        let mut entry = WordEntry::default();
        entry.pos = "noun".to_string();
        entry.translations = vec![
            Translation::new("es", "financial institution", "banco"),
            Translation::new("de", "financial institution", "Bank"),
            Translation::new("es", "sloping ground beside water", "banco"),
            Translation::new("de", "sloping ground beside water", "Sandbank"),
            Translation::new("es", "bench", "banco"),
            Translation::new("de", "bench", "Bank"),
        ];

        let mut irs = IGlossaryExtended::new();
        dict.process(langs, &entry, &mut irs);
        dict.postprocess(LangSpecs::from(langs), &mut irs);

        assert_eq!(irs.len(), 1);
        let (_, _, _, senses) = &irs[0];
        let senses: Vec<String> = senses
            .iter()
            .map(|(sense, translations)| format!("{sense}: {}", translations.join(", ")))
            .collect();
        assert_eq!(
            senses,
            [
                "financial institution: Bank",
                "sloping ground beside water: Sandbank",
                "bench: Bank",
            ]
        );

        // "bench" and "financial institution" share their German translation, so the three
        // senses render as two definitions.
        let yomitan_entries = to_yomitan_glossary_extended(langs.source, langs.target, &irs);
        let banco = yomitan_entries.first().unwrap();
        assert_eq!(banco.definitions.len(), 2);
    }

    #[test]
    fn process_glossary_extended_pos_localization() {
        let dict = DGlossaryExtended;
        // Japanese as target: the pos tag must come out localized
        let langs = Langs::new(Edition::En, Lang::En, Lang::Ja);
        let mut entry = WordEntry::default();
        entry.pos = "noun".to_string();
        entry.translations = vec![
            Translation::new("ja", "some sense", "日本語"),
            Translation::new("en", "some sense", "english"),
        ];

        let mut irs = IGlossaryExtended::new();
        dict.process(langs, &entry, &mut irs);

        assert_eq!(irs.len(), 1);
        let (_, _, pos, _) = &irs[0];

        assert_eq!(pos.long(), "noun");

        let yomitan_entries = to_yomitan_glossary_extended(langs.source, langs.target, &irs);
        let term_bank = yomitan_entries.first().unwrap();
        assert_eq!(term_bank.definition_tags[0].short_tag, "名");
    }
}
