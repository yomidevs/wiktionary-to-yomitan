//! Glossary and GlossaryExtended dictionaries.

use crate::{
    Map, Set,
    cli::{GlossaryArgs, GlossaryExtendedArgs, LangSpecs},
    dict::{Dictionary, Langs, main::get_reading},
    lang::{Edition, Lang},
    models::{
        kaikki::WordEntry,
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

impl Dictionary for DGlossaryExtended {
    type A = GlossaryExtendedArgs;
    type I = IGlossaryExtended;

    fn supports_probe(&self) -> bool {
        false
    }

    fn process(&self, langs: Langs, entry: &WordEntry, irs: &mut Self::I) {
        process_glossary_extended(langs.edition, langs.source, langs.target, entry, irs);
    }

    // TODO: change type "I" to not have to merge lemmas here
    fn postprocess(&self, irs: &mut Self::I) {
        let mut map = Map::default();

        for (lemma, pos, edition, translations) in irs.drain(..) {
            map.entry(lemma)
                .or_insert_with(|| (pos, edition, Set::default()))
                .2
                .extend(translations);
        }

        irs.extend(map.into_iter().map(|(lemma, (pos, edition, set))| {
            (lemma, pos, edition, set.into_iter().collect::<Vec<_>>())
        }));
    }

    fn to_yomitan(&self, langs: LangSpecs, irs: &Self::I) -> YomitanDict {
        YomitanDict::new(
            to_yomitan_glossary_extended(langs.target, irs),
            vec![],
            vec![],
        )
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

    let reading = get_reading(source, target, entry).unwrap_or_default();
    let definition_tags = match find_tag_in_bank(&entry.pos) {
        Some(mut tag_info) => {
            localize_tag_info(target, &mut tag_info);
            vec![tag_info]
        }
        None => vec![],
    };
    let pos = Pos::from(entry.pos.as_str());
    let rules = pos.short();

    irs.push(TermBankEntry::new(
        entry.word.clone(),
        reading,
        definition_tags,
        rules.to_string(),
        definitions,
    ));
}

/// (lemma, pos, edition, translations)
type IGlossaryExtended = Vec<(String, Pos, Edition, Vec<String>)>;

fn process_glossary_extended(
    edition: Edition,
    source: Lang,
    target: Lang,
    entry: &WordEntry,
    irs: &mut IGlossaryExtended,
) {
    let mut translations: Map<&str, (Vec<&str>, Vec<&str>)> = Map::default();

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
                .push(&translation.word);
        }
    }

    // We only keep translations with matches in both languages (source and target)
    translations.retain(|_, (targets, sources)| !targets.is_empty() && !sources.is_empty());

    if translations.is_empty() {
        return;
    }

    // A "semi" cartesian product. See the test below.
    irs.extend(translations.iter().flat_map(|(_, (targets, sources))| {
        sources.iter().map(|lemma| {
            (
                (*lemma).to_string(),
                Pos::from(entry.pos.as_str()),
                edition,
                targets.iter().map(|def| (*def).to_string()).collect(),
            )
        })
    }));
}

fn to_yomitan_glossary_extended(target: Lang, irs: &IGlossaryExtended) -> Vec<TermBankEntry> {
    irs.iter()
        .map(|(lemma, pos, _, translations)| {
            let definition_tags = match find_tag_in_bank(pos.long()) {
                Some(mut tag_info) => {
                    localize_tag_info(target, &mut tag_info);
                    vec![tag_info]
                }
                None => vec![],
            };
            let rules = pos.short();

            TermBankEntry::new(
                lemma.clone(),
                String::new(),
                definition_tags,
                rules.to_string(),
                translations
                    .iter()
                    .cloned()
                    .map(DetailedDefinition::Text)
                    .collect(),
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

        let (lemma1, pos, _, defs1) = &irs[0];
        let (lemma2, _, _, defs2) = &irs[1];
        let (lemma3, _, _, defs3) = &irs[2];

        assert_eq!(pos.long(), "noun");
        assert_eq!(lemma1, "Ἡράκλειαι στῆλαι");
        assert_eq!(lemma2, "Ἡράκλειαι στῆλαι");
        assert_eq!(lemma3, "Κάλπη");

        let expected = vec!["Gibraltar".to_string(), "Gjibraltari".to_string()];
        assert_eq!(defs1, &expected);
        assert_eq!(defs2, &expected);
        assert_eq!(defs3, &expected);

        dict.postprocess(&mut irs);
        assert_eq!(irs.len(), 2);

        let yomitan_entries = to_yomitan_glossary_extended(Lang::Grc, &irs);
        assert_eq!(yomitan_entries.len(), 2);
        let term_bank = yomitan_entries.first().unwrap();
        assert_eq!(term_bank.definition_tags[0].short_tag, "n");
    }

    #[test]
    fn process_glossary_extended_pos_localization() {
        let dict = DGlossaryExtended;
        // Japanese as target, source irrelevant
        let langs = Langs::new(Edition::En, Lang::Ja, Lang::En);
        let mut entry = WordEntry::default();
        entry.pos = "noun".to_string();
        entry.translations = vec![
            Translation::new("ja", "some sense", "日本語"),
            Translation::new("en", "some sense", "english"),
        ];

        let mut irs = IGlossaryExtended::new();
        dict.process(langs, &entry, &mut irs);

        assert_eq!(irs.len(), 1);
        let (_, pos, _, _) = &irs[0];

        assert_eq!(pos.long(), "noun");

        let yomitan_entries = to_yomitan_glossary_extended(Lang::Ja, &irs);
        let term_bank = yomitan_entries.first().unwrap();
        assert_eq!(term_bank.definition_tags[0].short_tag, "名");
    }
}
