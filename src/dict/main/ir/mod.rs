use std::{fs::File, io::BufWriter, path::PathBuf, sync::LazyLock};

use anyhow::Result;
use indexmap::map::Entry;
use regex::Regex;
use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

mod heap;
use heap::HeapSize;

mod preprocess_forms;
use preprocess_forms::preprocess_forms;

mod postprocess;
pub use postprocess::postprocess_main;

use crate::{
    Map, Set,
    cli::{LangSpecs, Options},
    dict::Intermediate,
    lang::{Edition, Lang},
    models::kaikki::{Example, Form, HeadTemplate, Sense, Synonym, Tag, WordEntry},
    path::PathManager,
    tags::{Pos, REDUNDANT_FORM_TAGS},
    utils::{human_size, link_kaikki, link_wiktionary},
};

const MAX_NUMBER_OF_SYNONYMS: usize = 3;
const MAX_NUMBER_OF_EXAMPLES: usize = 3;
const MAX_SIZE_OF_EXAMPLE: usize = 120;
const MAX_SIZE_OF_EXAMPLE_REFERENCE: usize = 120;

/// Intermediate representation of the main dictionary.
#[derive(Debug, Default)]
pub struct Tidy {
    pub lemma_map: LemmaMap, // 56
    pub form_map: FormMap,   // 56
}

impl Intermediate for Tidy {
    fn len(&self) -> usize {
        self.len()
    }

    fn write(&self, pm: &PathManager) -> Result<PathBuf> {
        self.write(pm)
    }
}

impl Tidy {
    fn len(&self) -> usize {
        self.lemma_map.len() + self.form_map.len()
    }

    fn insert_lemma(&mut self, lemma: &str, reading: &str, pos: &str, entry: LemmaInfo) {
        self.lemma_map.insert(lemma, reading, pos, entry);
    }

    fn insert_form(
        &mut self,
        uninflected: &str,
        inflected: &str,
        pos: &str,
        source: FormSource,
        tags: Vec<Tag>,
    ) {
        self.form_map
            .insert(uninflected, inflected, pos, source, tags);
    }

    #[tracing::instrument(skip_all)]
    fn write(&self, pm: &PathManager) -> Result<PathBuf> {
        let dir_tidy = pm.dir_tidy();
        _ = std::fs::create_dir_all(&dir_tidy);

        if !self.lemma_map.0.is_empty() {
            self.write_json(pm.path_lemmas(), &self.lemma_map, pm.opts.pretty)?;
        }
        if !self.form_map.0.is_empty() {
            self.write_json(pm.path_forms(), &self.form_map, pm.opts.pretty)?;
        }

        Ok(dir_tidy)
    }

    fn write_json<T: Serialize>(&self, path: PathBuf, data: &T, pretty: bool) -> Result<()> {
        let writer = BufWriter::new(File::create(&path)?);
        if pretty {
            serde_json::to_writer_pretty(writer, data)?;
        } else {
            serde_json::to_writer(writer, data)?;
        }
        Ok(())
    }
}

pub fn found_ir_message_impl(langs: LangSpecs, irs: &Tidy) {
    let n_lemmas = irs.lemma_map.len();
    let n_forms = irs.form_map.len();
    let n_irs = n_lemmas + n_forms;
    let n_forms_inflection = irs.form_map.len_inflection();
    let n_forms_extracted = irs.form_map.len_extracted();
    let n_forms_alt_of = irs.form_map.len_alt_of();

    debug_assert_eq!(
        n_forms,
        n_forms_inflection + n_forms_extracted + n_forms_alt_of,
        "mismatch in form counts"
    );

    let lemma_heap = irs.lemma_map.heap_size() as f64;
    let form_heap = irs.form_map.heap_size() as f64;
    let irs_heap = lemma_heap + form_heap;
    let lemma_heap_msg = human_size(lemma_heap);
    let form_heap_msg = human_size(form_heap);
    let irs_heap_msg = human_size(irs_heap);

    const MB: f64 = 1024.0 * 1024.0;
    if irs_heap > 500.0 * MB {
        tracing::debug!(
            "[{}-{}] Found {} irs ({})",
            langs.source,
            langs.target,
            n_irs,
            irs_heap_msg,
        );
        tracing::debug!("├─ terms: {} ({})", n_lemmas, lemma_heap_msg,);
        tracing::debug!(
            "└─ forms: {} ({}) [infl {}, extr {}, alt {}]",
            n_forms,
            form_heap_msg,
            n_forms_inflection,
            n_forms_extracted,
            n_forms_alt_of,
        );
    } else {
        tracing::debug!(
            "Found {n_irs} irs: {n_lemmas} terms, {n_forms} forms \
                [{n_forms_inflection} infl, {n_forms_extracted} extr, {n_forms_alt_of} alt]"
        );
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct LemmaKey {
    lemma: String,
    reading: String,
    pos: Pos,
}

impl LemmaKey {
    fn new(lemma: &str, reading: &str, pos: Pos) -> Self {
        Self {
            lemma: lemma.into(),
            reading: reading.into(),
            pos,
        }
    }

    fn unpack(&self) -> (&str, &str, Pos) {
        (&self.lemma, &self.reading, self.pos)
    }
}

#[derive(Debug, Default)]
pub struct LemmaMap(Map<LemmaKey, Vec<LemmaInfo>>);

impl LemmaMap {
    fn insert(&mut self, lemma: &str, reading: &str, pos: &str, entry: LemmaInfo) {
        debug_assert!(!entry.gloss_tree.is_empty());

        let key = LemmaKey::new(lemma, reading, Pos::from(pos));

        match self.0.entry(key) {
            Entry::Vacant(e) => {
                e.insert(vec![entry]);
            }
            Entry::Occupied(mut e) => {
                e.get_mut().push(entry);
            }
        }
    }
}

// We only serialize for debugging in the testsuite, so having this tmp nested is easy to write and
// has no overhead when building the dictionary without --save-temps. This way, we avoid storing
// nested structures that are less performant (both for cache locality, and number of lookups).
impl Serialize for LemmaMap {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut nested: Map<&str, Map<&str, Map<Pos, &Vec<LemmaInfo>>>> = Map::default();

        for (key, infos) in &self.0 {
            let (lemma, reading, pos) = key.unpack();
            nested
                .entry(lemma)
                .or_default()
                .entry(reading)
                .or_default()
                .insert(pos, infos);
        }

        nested.serialize(serializer)
    }
}

impl LemmaMap {
    pub fn flat_iter(&self) -> impl Iterator<Item = (&str, &str, Pos, &LemmaInfo)> {
        self.0.iter().flat_map(|(key, infos)| {
            infos.iter().map(move |info| {
                let (lemma, reading, pos) = key.unpack();
                (lemma, reading, pos, info)
            })
        })
    }

    fn len(&self) -> usize {
        self.0.values().map(Vec::len).sum()
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct FormKey {
    uninflected: String,
    inflected: String,
    pos: Pos,
}

impl FormKey {
    fn new(uninflected: &str, inflected: &str, pos: Pos) -> Self {
        Self {
            uninflected: uninflected.into(),
            inflected: inflected.into(),
            pos,
        }
    }

    const fn unpack(&self) -> (&str, &str, Pos) {
        (self.uninflected.as_str(), self.inflected.as_str(), self.pos)
    }
}

#[derive(Debug, Default)]
pub struct FormMap(Map<FormKey, (FormSource, Vec<String>)>);

impl FormMap {
    fn insert(
        &mut self,
        uninflected: &str,
        inflected: &str,
        pos: &str,
        source: FormSource,
        tags: Vec<Tag>,
    ) {
        // There are too many callers of this function: better check it here.
        if tags.is_empty()
            || uninflected.is_empty()
            || inflected.is_empty()
            || uninflected == inflected
        {
            return;
        }

        let key = FormKey::new(uninflected, inflected, Pos::from(pos));

        match self.0.entry(key) {
            Entry::Vacant(e) => {
                e.insert((source, tags));
            }
            Entry::Occupied(mut e) => {
                e.get_mut().1.extend(tags);
            }
        }
    }
}

// We only serialize for debugging in the testsuite, so having this tmp nested is easy to write and
// has no overhead when building the dictionary without --save-temps. This way, we avoid storing
// nested structures that are less performant (both for cache locality, and number of lookups).
impl Serialize for FormMap {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[expect(clippy::type_complexity)]
        let mut nested: Map<&str, Map<&str, Map<Pos, &(FormSource, Vec<String>)>>> = Map::default();

        for (key, infos) in &self.0 {
            let (uninflected, inflected, pos) = key.unpack();
            nested
                .entry(uninflected)
                .or_default()
                .entry(inflected)
                .or_default()
                .insert(pos, infos);
        }

        nested.serialize(serializer)
    }
}

impl FormMap {
    /// Iterates over: uninflected, inflected, pos, source, tags
    pub fn flat_iter(&self) -> impl Iterator<Item = (&str, &str, Pos, &FormSource, &Vec<String>)> {
        self.0.iter().map(|(key, (source, tags))| {
            let (uninflected, inflected, pos) = key.unpack();
            (uninflected, inflected, pos, source, tags)
        })
    }

    /// Iterates over: uninflected, inflected, pos, source, tags
    pub fn flat_iter_mut(
        &mut self,
    ) -> impl Iterator<Item = (&str, &str, Pos, &mut FormSource, &mut Vec<String>)> {
        self.0.iter_mut().map(|(key, (source, tags))| {
            let (uninflected, inflected, pos) = key.unpack();
            (uninflected, inflected, pos, source, tags)
        })
    }

    fn len(&self) -> usize {
        self.flat_iter().count()
    }

    fn len_of(&self, source: FormSource) -> usize {
        self.flat_iter()
            .filter(|(_, _, _, src, _)| **src == source)
            .count()
    }

    fn len_extracted(&self) -> usize {
        self.len_of(FormSource::Extracted)
    }

    fn len_inflection(&self) -> usize {
        self.len_of(FormSource::Inflection)
    }

    fn len_alt_of(&self) -> usize {
        self.len_of(FormSource::AltOfTop) + self.len_of(FormSource::AltOfSense)
    }
}

/// Enum used exclusively for debugging. This information doesn't appear on the dictionary.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FormSource {
    /// Form extracted from `entry.forms`
    Extracted,
    /// Form added via gloss analysis ("is inflection of...")
    Inflection,
    /// Alternative forms (top-level and sense-level)
    AltOfTop,
    AltOfSense,
}

// NOTE: the less we have here the better. For example, the links could be entirely moved to the
// yomitan side of things. It all depends on what we may or may not consider useful for debugging.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LemmaInfo {
    pub gloss_tree: GlossTree,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<Tag>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub synonyms: Vec<Synonym>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub etymology_text: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub head_info_text: Option<String>,

    #[serde(rename = "wlink")]
    pub link_wiktionary: String,

    #[serde(rename = "klink")]
    pub link_kaikki: String,
}

pub type GlossTree = Map<String, GlossInfo>;

// ... its really SenseInfo but oh well
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
#[serde(default)]
pub struct GlossInfo {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<Tag>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub topics: Vec<Tag>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub examples: Vec<Example>,

    /// Children glosses (sub-senses).
    /// Boxed to avoid paying for a full [`GlossTree`] allocation (a [`Map`])
    /// on every leaf node - which is the common case.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Box<GlossTree>>,
}

pub fn process_main(edition: Edition, source: Lang, entry: &WordEntry, irs: &mut Tidy) {
    process_forms(edition, source, entry, irs);

    process_alt_forms(entry, irs);

    let reading = match get_reading(edition, source, entry) {
        Some(reading) if reading != entry.word => reading,
        _ => String::new(),
    };

    if entry.contains_no_gloss() {
        process_no_gloss(edition, entry, irs);
    } else {
        irs.insert_lemma(
            &entry.word,
            &reading,
            &entry.pos,
            process_entry(edition, source, entry),
        );
    }
}

/// Whether we should completely skip this entry.
///
/// The function is trivial at the moment and only relevant for the [ja-en] dict.
pub fn should_skip_entry(entry: &WordEntry) -> bool {
    // https://en.wiktionary.org/wiki/toraware#Japanese
    entry.pos == "romanization"
}

// Everything that mutates entry
pub fn preprocess_main(
    edition: Edition,
    source: Lang,
    opts: &Options,
    entry: &mut WordEntry,
    irs: &mut Tidy,
) {
    // WARN:: mutates entry::forms (and entry::forms::form)
    preprocess_forms(edition, source, entry);

    // WARN: mutates entry::senses::sense::tags
    match edition {
        Edition::En => {
            // The original fetched them from head_templates but it is better not to touch that
            // and we can do the same by looking at the tags of the canonical form.
            if let Some(cform) = entry.canonical_form() {
                let cform_tags: Vec<_> = cform.tags.clone();
                for sense in &mut entry.senses {
                    for tag in &cform_tags {
                        if tag != "canonical" && !sense.tags.contains(tag) {
                            sense.tags.push(tag.into());
                        }
                    }
                }
            }
        }
        Edition::El => {
            // Fetch gender from a matching form
            let gender_tags = ["masculine", "feminine", "neuter"];
            for form in &entry.forms {
                if form.form == entry.word {
                    for sense in &mut entry.senses {
                        for tag in &form.tags {
                            if gender_tags.contains(&tag.as_str()) && !sense.tags.contains(tag) {
                                sense.tags.push(tag.into());
                            }
                        }
                    }
                }
            }
        }
        _ => (),
    }

    // WARN: mutates entry::senses
    //
    // Deal with "no definition" glosses, cf. https://it.wiktionary.org/wiki/cartoccio#Italian
    // That is, glosses that are of no value, usually of the shape "Empty definition, add one at
    // this link etc."
    //
    // See also: https://it.wiktionary.org/wiki/Template:Nodef
    if edition == Edition::It {
        for sense in &mut entry.senses {
            sense
                .glosses
                .retain(|gloss| *gloss != "definizione mancante; se vuoi, aggiungila tu");
        }
    }

    // WARN: mutates entry::senses
    //
    // What if the current word is an inflection but *also* has an inflection table?
    // https://el.wiktionary.org/wiki/ψηφίσας
    //
    // That is, imagine participle A comes from verb B, but A is treated as an adjective, so
    // it has a declension table. If we are not careful, every word C in the table that is a form
    // of A will not appear in the dictionary!
    //
    // It does not happen in English, but bear with this fake example:
    // * C = runnings < A = running < B = run
    // then, by saying that A is just a form of B, we will remove the sense, and the entry won't be
    // added to lemmas because there are no senses at all. All the information in the declension
    // table saying C < A will yield no results. Effectively, hovering over C in yomitan will show
    // nothing. Not ideal.
    //
    // There are two choices, make C point to B, or keep A as a non-lemma. We opt for the latter,
    // checking that there are no trivial forms (C) in WordEntry. Only then we can safely delete
    // the sense.
    //
    // Note that deleting senses is a good decision overall: it reduces clutter and forces the
    // redirect. One just has to be careful about when to do it
    //
    let old_senses = std::mem::take(&mut entry.senses);
    let mut senses_without_inflections = Vec::new();
    for sense in old_senses {
        if (!opts.experimental || entry.non_trivial_forms().next().is_none())
            && handle_inflection_sense(edition, source, entry, &sense, irs)
        {
            // handled as inflection
        } else if handle_alt_of_sense(entry, &sense, irs) {
            // handled as alt-of
        } else if handle_see_sense(edition, entry, &sense, irs) {
            // handled as see
        } else {
            senses_without_inflections.push(sense);
        }
    }
    entry.senses = senses_without_inflections;

    // WARN: mutates entry::senses::glosses
    //
    // rg: full stop
    // https://github.com/yomidevs/yomitan/issues/2232
    // Add an empty whitespace at the end... and it works!
    if opts.experimental {
        static TRAILING_PUNCT_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"\p{P}$").unwrap());
        for sense in &mut entry.senses {
            for gloss in &mut sense.glosses {
                if !TRAILING_PUNCT_RE.is_match(gloss) {
                    gloss.push(' ');
                }
            }
        }
    }
}

/// Add Extracted forms. That is, forms from `entry.forms`.
fn process_forms(edition: Edition, source: Lang, entry: &WordEntry, irs: &mut Tidy) {
    for form in entry.non_trivial_forms() {
        debug_assert_ne!(form.form, entry.word);

        if should_break_at_finish_forms(edition, source, form) {
            break;
        }

        if should_skip_form(edition, source, entry, form) {
            continue;
        }

        let filtered_tags = form
            .tags
            .iter()
            .map(String::as_str)
            .filter(|tag| !REDUNDANT_FORM_TAGS.contains(tag))
            .collect::<Vec<_>>()
            .join(" ");

        irs.insert_form(
            &entry.word,
            &form.form,
            &entry.pos,
            FormSource::Extracted,
            vec![filtered_tags],
        );
    }
}

// This requires knowledge of witktionary table structures, the edition language and Yomitan.
// The point being to skip unnecessary forms because either:
// - They are already included in a shorter form which is equally useful
//   Ex. if we have the fr "arrive", don't add "j'arrive"
// - (This for later) Yomitan has inflection rules that make adding the form pointless
//   Ex. "runs" for "run", since there is already a plural rule
// - They are useless
//   - forms with multiple articles (useless for a dictionary). Ex. "il/elle/on arrive"
//   - most composite verbs are in most languages
//
// Eventually it would be preferable if these were done at wiktextract level, but let's do the work
// ourselves for now
fn should_skip_form(edition: Edition, source: Lang, entry: &WordEntry, form: &Form) -> bool {
    match (edition, source) {
        (Edition::Fr, Lang::Fr) => {
            // Objectively better
            if ["qu’", "que ", "en "]
                .iter()
                .any(|p| form.form.starts_with(p))
            {
                return true;
            }
        }
        (Edition::En, Lang::Ja) => {
            // Skip romanization: "hashireru", "tanoshikarō" etc.
            if is_japanese_romanization(&form.form) {
                return true;
            }
            // Sometimes romanization is at the end: 走っていません [hashitte imasen]
            // (which seems like a parsing error on compound-word by wiktextract)
            // This should be safe enough: I can't think of a useful form with brackets.
            if form.form.contains('[') && form.form.contains(']') {
                return true;
            }
        }
        (Edition::En, Lang::En) => {
            // Bloated, remove rare variants
            if form
                .tags
                .iter()
                .any(|tag| tag == "rare" || tag == "nonstandard" || tag == "dialectal")
            {
                return true;
            }

            // "more expansive" ~ forms are useless, hovering "expansive" is enough
            if form.form.contains(' ')
                && form
                    .tags
                    .iter()
                    .any(|tag| tag == "comparative" || tag == "superlative")
            {
                return true;
            }
        }
        (Edition::En, Lang::Fi) => {
            // Bloated, remove anything non-essential
            // For the reasoning behind possessive, see. should_break_at_finish_forms
            if form
                .tags
                .iter()
                .any(|tag| tag == "rare" || tag == "possessive")
            {
                return true;
            }
            // We don't support composite forms
            // https://en.wiktionary.org/wiki/pullistaa
            if form.form.contains(' ') {
                return true;
            }
        }
        (Edition::Ja, Lang::Ja) => {
            // Since we expect "Letter" as Yomitan "Dictionary search resolution", forms prefixed
            // by the lemma are redundant.
            // - lemma: ぷくぷく
            // - forms: ぷくぷくし | ぷくぷくせ | ぷくぷくさ etc.
            if form.form.starts_with(&entry.word) {
                return true;
            }
        }
        _ => (),
    }
    false
}

fn is_japanese_romanization(form: &str) -> bool {
    form.chars()
        .all(|c| c.is_ascii() || matches!(c, 'ā' | 'ī' | 'ū' | 'ē' | 'ō'))
}

// Finnish from the English edition crashes with out-of-memory.
// There are simply too many forms, so we prune the less used (possessive).
//
// https://uusikielemme.fi/finnish-grammar/possessive-suffixes-possessiivisuffiksit#one
fn should_break_at_finish_forms(edition: Edition, source: Lang, form: &Form) -> bool {
    if matches!((edition, source), (Edition::En, Lang::Fi)) {
        // HACK: 1. For tables that parse the title
        // https://kaikki.org/dictionary/Finnish/meaning/p/p%C3%A4/p%C3%A4%C3%A4.html
        if form.form == "See the possessive forms below." {
            return true;
        }
        // HACK: 2. For tables that don't parse the title
        // https://kaikki.org/dictionary/Finnish/meaning/i/is/iso.html
        // https://github.com/tatuylonen/wiktextract/issues/1565
        if form.form == "Rare. Only used with substantive adjectives." {
            return true;
        }
    }
    false
}

/// Add `AltOf` forms. That is, alternative forms.
fn process_alt_forms(entry: &WordEntry, irs: &mut Tidy) {
    for alt_form in &entry.alt_of {
        irs.insert_form(
            &entry.word,
            &alt_form.word,
            &entry.pos,
            FormSource::AltOfTop,
            vec!["alt-of".to_string()],
        );
    }
}

/// Process "no-gloss" word entries for alternative ways of adding lemmas/forms.
#[expect(clippy::single_match)]
fn process_no_gloss(edition: Edition, entry: &WordEntry, irs: &mut Tidy) {
    match edition {
        // Unfortunately we are in the same A from B, B from C situation discussed in
        // preprocess_entry. There is no easy solution for adding the lemma back because at
        // this point the gloss has been deleted. Maybe reconsider the original approach of
        // deleting glosses, and mark them somehow as "inflection-only".
        //
        // At any rate, this will still add useful redirections.
        Edition::El => {
            if entry.is_participle()
                && let Some(form_of) = entry.form_of.first()
            {
                irs.insert_form(
                    &form_of.word,
                    &entry.word,
                    &entry.pos,
                    FormSource::Inflection,
                    vec![format!("redirected from {}", entry.word)],
                );
            }
        }
        _ => (),
    }
}

// There are potentially more than one, but yomitan doesn't really support displaying
// multiple readings at the same time (other than adding a copy of the yomitan entry).
//
// For Japanese, if there are multiple readings, they are added as forms, but the reading
// displayed over the word is always the first, which is not what we want if we search
// an alternative reading. F.e. in 鉛粉 (えんぷん 又は なまりふん), searching
// なまりふん will display 鉛粉 with furigana えんぷん.
// It is relatively rare (since extra readings are rare), and as such, it has low priority.
pub fn get_reading(edition: Edition, source: Lang, entry: &WordEntry) -> Option<String> {
    match (edition, source) {
        (Edition::En, Lang::Ja) => get_japanese_reading(entry),
        (Edition::En, Lang::Fa) => entry.romanization_form().map(|f| f.form.clone()),
        (Edition::Ja, _) => entry.transliteration_form().map(|f| f.form.clone()),
        (Edition::En | Edition::Zh, Lang::Zh) => entry.pinyin().map(String::from),
        _ => get_canonical_word(source, entry),
    }
}

/// The canonical word may contain extra diacritics.
///
/// For most languages, this is equal to word, but for, let's say, Latin, there may be a
/// difference (cf. <https://en.wiktionary.org/wiki/fama>, where `entry.word` is fama, but
/// this will return fāma).
fn get_canonical_word(source: Lang, entry: &WordEntry) -> Option<String> {
    match source {
        Lang::La | Lang::Ru | Lang::Grc | Lang::Ar | Lang::Fa => {
            entry.canonical_form().map(|f| f.form.clone())
        }
        _ => None,
    }
}

// TODO: work with this to return a &str
//
// Does not support multiple readings
fn get_japanese_reading(entry: &WordEntry) -> Option<String> {
    // The original parses head_templates directly (which probably deserves a PR to
    // wiktextract), although imo pronunciation templates should have been better.
    // There is no pronunciation template info in en-wiktextract, and while I think that
    // information ends up in sounds, it is not always reliable. For example:
    // https://en.wiktionary.org/wiki/お腹が空いた
    // https://kaikki.org/dictionary/Japanese/meaning/お/お腹/お腹が空いた.html
    // has a pronunciation template:
    // {{ja-pron|おなか が すいた}}
    // but no "other" sounds, which is where pronunciations are usually stored.

    // Ideally we would just do this:
    // for sound in &entry.sounds {
    //     if !sound.other.is_empty() {
    //         return &sound.other;
    //     }
    // }

    // I really don't want to touch templates so instead, replace the ruby
    if let Some(cform) = entry.canonical_form()
        && !cform.ruby.is_empty()
    {
        // https://github.com/tatuylonen/wiktextract/issues/1484
        // let mut cform_lemma = cform.form.clone();
        // if cform_lemma != entry.word {
        //     warn!(
        //         "Canonical form: '{cform_lemma}' != word: '{}'\n{}\n{}\n\n",
        //         entry.word,
        //         link_wiktionary(args, &entry.word),
        //         link_kaikki(args, &entry.word),
        //     );
        // } else {
        //     warn!(
        //         "Equal for word: '{}'\n{}\n{}\n\n",
        //         entry.word,
        //         link_wiktionary(args, &entry.word),
        //         link_kaikki(args, &entry.word),
        //     );
        // }

        // This should be cform.form, but it's not parsed properly:
        // https://github.com/tatuylonen/wiktextract/issues/1484
        let mut cform_lemma = entry.word.clone();
        let mut cursor = 0;
        for (base, reading) in &cform.ruby {
            if let Some(pos) = cform_lemma[cursor..].find(base) {
                let start = cursor + pos;
                let end = start + base.len();
                cform_lemma.replace_range(start..end, reading);
                cursor = start + reading.len();
            } else {
                tracing::warn!("Kanji '{}' not found in '{}'", base, cform_lemma);
                return None;
            }
        }
        return Some(cform_lemma);
    }

    None
}

fn process_entry(edition: Edition, source: Lang, entry: &WordEntry) -> LemmaInfo {
    LemmaInfo {
        gloss_tree: get_gloss_tree(entry),
        tags: entry.tags.clone(),
        synonyms: entry
            .synonyms
            .iter()
            .filter(|syn| syn.word != entry.word)
            .take(MAX_NUMBER_OF_SYNONYMS)
            .cloned()
            .collect(),
        etymology_text: entry
            .etymology_texts()
            // TODO: patch this in wiktextract
            .filter(|texts| match edition {
                // "Missing etymology" placeholder: remove it
                // We can't do it at preprocess_main because of the opaqueness of etymology text due to
                // some editions using a String (etymology text), and others a Vec (etymology textS)
                Edition::Ru => !texts.contains(&"Происходит от ??"),
                _ => true,
            })
            .map(|etymology_text| etymology_text.join("\n")),
        head_info_text: get_head_info(&entry.head_templates)
            .map(|head_info_text| head_info_text.join("\n")),
        link_wiktionary: link_wiktionary(edition, source, &entry.word),
        link_kaikki: link_kaikki(edition, source, &entry.word),
    }
}

static PARENS_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\(.+?\)").unwrap());

// For consistency, it's better if this function shares return type with .etymology_texts()
fn get_head_info(head_templates: &[HeadTemplate]) -> Option<Vec<&str>> {
    // It's rare, but duplicates can happen:
    // https://kaikki.org/dictionary/Irish/meaning/p/po/postáil.html
    let mut seen = Set::default();
    let result: Vec<_> = head_templates
        .iter()
        .filter_map(|head_template| {
            let expansion = head_template.expansion.as_str();
            if PARENS_RE.is_match(expansion) && seen.insert(expansion) {
                Some(expansion)
            } else {
                None
            }
        })
        .collect();

    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

fn get_gloss_tree(entry: &WordEntry) -> GlossTree {
    let mut gloss_tree = GlossTree::default();

    for sense in &entry.senses {
        let mut filtered_examples: Vec<_> = sense
            .examples
            .iter()
            .filter(|ex| !ex.text.is_empty() && ex.text.chars().count() <= MAX_SIZE_OF_EXAMPLE)
            .take(MAX_NUMBER_OF_EXAMPLES)
            .cloned()
            .map(|mut ex| {
                // Remove reference if too long
                if ex.reference.chars().count() > MAX_SIZE_OF_EXAMPLE_REFERENCE {
                    ex.reference = String::new();
                }
                ex
            })
            .collect();
        // Place examples with translations first
        filtered_examples.sort_by_key(|ex| ex.translation.is_empty());

        insert_glosses(
            &mut gloss_tree,
            &sense.glosses,
            &sense.tags,
            &sense.topics,
            &filtered_examples,
        );
    }

    gloss_tree
}

/// Recursive helper to deal with nested glosses
fn insert_glosses(
    gloss_tree: &mut GlossTree,
    glosses: &[String],
    tags: &[Tag],
    topics: &[Tag],
    examples: &[Example],
) {
    let Some(head) = glosses.first() else {
        return;
    };

    let tail = &glosses[1..];

    // get or insert node with only tags at this level
    let node = gloss_tree.entry(head.clone()).or_insert_with(|| GlossInfo {
        tags: tags.to_vec(),
        topics: topics.to_vec(),
        examples: Vec::new(),
        children: None,
    });

    // intersect tags if node already exists
    if !node.tags.is_empty() {
        node.tags = tags
            .iter()
            .filter(|&t| node.tags.contains(t))
            .cloned()
            .collect();
    }

    // assign examples to the last level
    if tail.is_empty() {
        node.examples = examples.to_vec();
        return;
    }

    let children = node
        .children
        .get_or_insert_with(|| Box::new(GlossTree::default()));
    insert_glosses(children, tail, tags, topics, examples);
}

static DE_INFLECTION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^(.*)des (?:Verbs|Adjektivs|Substantivs|Demonstrativpronomens|Possessivpronomens|Pronomens) (.*)$"
    ).unwrap()
});

const TAGS_RETAINED_EL: [&str; 9] = [
    "masculine",
    "feminine",
    "neuter",
    "singular",
    "plural",
    "nominative",
    "accusative",
    "genitive",
    "vocative",
];

/// Returns `true` if the sense is an inflection and was handled (inserted as form).
fn handle_inflection_sense(
    edition: Edition,
    source: Lang,
    entry: &WordEntry,
    sense: &Sense,
    irs: &mut Tidy,
) -> bool {
    if sense.glosses.is_empty() {
        return false;
    }

    match edition {
        Edition::De => {
            let Some((inflection_tags, uninflected)) = sense
                .glosses
                .first()
                .and_then(|gloss| DE_INFLECTION_RE.captures(gloss))
                .and_then(|caps| Some((caps.get(1)?, caps.get(2)?)))
            else {
                return false;
            };
            let inflection_tags = inflection_tags.as_str().trim();
            irs.insert_form(
                uninflected.as_str(),
                &entry.word,
                &entry.pos,
                FormSource::Inflection,
                vec![inflection_tags.to_string()],
            );
            true
        }
        Edition::El => {
            if sense.form_of.is_empty() || !sense.glosses.iter().any(|g| g.contains("του")) {
                return false;
            }
            let allowed_tags: Vec<_> = sense
                .tags
                .iter()
                .filter(|tag| TAGS_RETAINED_EL.contains(&tag.as_str()))
                .map(String::from)
                .collect();
            let inflection_tags: Vec<_> = if allowed_tags.is_empty() {
                // very rare
                vec![format!("redirected from {}", entry.word)]
            } else {
                allowed_tags
            };
            for form in &sense.form_of {
                irs.insert_form(
                    &form.word,
                    &entry.word,
                    &entry.pos,
                    FormSource::Inflection,
                    // Unfortunate clone. Most sense.form_of only contain one form...
                    inflection_tags.clone(),
                );
            }
            true
        }
        Edition::En => {
            let is_inflection = sense.glosses.iter().any(|gloss| {
                if gloss.contains("inflection of") {
                    return true;
                }
                for form in &sense.form_of {
                    if form.word.is_empty() {
                        continue;
                    }
                    // We are looking for "... of {word}$" or "... of {word} (text)$"
                    //
                    // Cf.
                    // ... imperative of iki
                    // ... perfective of возни́кнуть (vozníknutʹ)
                    // But no
                    // ... agent noun of fahren; driver (person)
                    let subs = format!("of {}", form.word);
                    if gloss.ends_with(&subs)
                        || (gloss.contains(&format!("{subs} (")) && gloss.ends_with(')'))
                    {
                        return true;
                    }
                }
                false
            });
            if !is_inflection {
                return false;
            }
            handle_inflection_sense_en(source, entry, sense, irs);
            true
        }
        // (This comment talks about the Italian edition since it was the first implementation)
        //
        // Cf. https://kaikki.org/itwiktionary/Italiano/meaning/s/sc/scorrevo.html
        //
        // This is the most generic way of dealing with inflection, but assumes that the
        // edition is parsed by kaikki with "form_of" at sense level. It could be extended to
        // other languages.
        //
        // Note: because of how "etymologies" (read, definitions) are processed by Kaikki, we
        // are guaranteed that if we find an inflection, all the glosses are variations of that
        // inflection.
        // This means that a word like "cura", that both in Spanish and Italian is both a verb
        // and a noun, will not have the glosses intermingled: we will only find glosses for
        // the verb in the verb "etymology", and similarly for the noun.
        // Therefore, we can safely assume that, if we have a form_of, every gloss is an
        // explanation about the inflection, and that the only condition we really care about,
        // is the presence of a "form_of".
        //
        // Cf. https://it.wiktionary.org/wiki/cura#Italiano (fixed recently)
        // Cf. https://it.wiktionary.org/wiki/ceni#Italiano
        _ => {
            if sense.form_of.len() != 1 {
                return false;
            }
            // One could use sense::glosses as tags, but while they carry important
            // information, they are obscenely verbose.
            let form_of = &sense.form_of[0];
            // Some redirections may not contain the "form-of" tag at sense level!
            // https://kaikki.org/jawiktionary/日本語/meaning/名/名前/名前.html
            debug_assert!(
                sense.tags.iter().any(|tag| *tag == "form-of")
                    || entry.tags.iter().any(|tag| *tag == "form-of")
            );
            let allowed_tags: Vec<_> = sense
                .tags
                .iter()
                .filter(|tag| *tag != "form-of")
                .map(String::from)
                .collect();
            let inflection_tags: Vec<_> = if allowed_tags.is_empty() {
                vec![format!("redirected from {}", entry.word)]
            } else {
                allowed_tags
            };
            // We need to normalize, for instance, for the la-ja, so that appellantur redirects to
            // appellare and not to appellāre (since only appellare appears as lemma)
            let norm_form_of_word = normalize_orthography(source, &form_of.word);
            irs.insert_form(
                &norm_form_of_word,
                &entry.word,
                &entry.pos,
                FormSource::Inflection,
                inflection_tags,
            );
            true
        }
    }
}

fn handle_inflection_sense_en(source: Lang, entry: &WordEntry, sense: &Sense, irs: &mut Tidy) {
    let uninflected = match sense.form_of.as_slice() {
        [alt_form] => &alt_form.word,
        _ => return,
    };

    // Not sure if this is better (cf. ru-en) over entry.word but it is what was done in
    // the original, so lets not change that for the moment.
    let inflected = get_canonical_word(source, entry).unwrap_or_else(|| entry.word.clone());

    if inflected == *uninflected {
        return;
    }

    let mut inflections = Set::default();
    let of_uninflected = format!("of {uninflected}");
    for gloss in &sense.glosses {
        let cleaned = gloss
            .replace("inflection of ", "")
            .replace(&of_uninflected, "")
            .replace(uninflected, "")
            .replace(':', "");

        let inflection = PARENS_RE.replace_all(&cleaned, "").trim().to_string();

        if !inflection.is_empty() {
            inflections.insert(inflection);
        }
    }

    for inflection in inflections {
        irs.insert_form(
            uninflected,
            &inflected,
            &entry.pos,
            FormSource::Inflection,
            vec![inflection],
        );
    }
}

fn handle_alt_of_sense(entry: &WordEntry, sense: &Sense, irs: &mut Tidy) -> bool {
    let mut handled = false;
    // We rely on Wiktionary having a page for the alt_form, otherwise this can potentially
    // remove useful glosses. Then again, the same applies to handle_inflection_sense.
    //
    // The issue with alt_of is that the best redirection order is not granted, while for
    // form_of is easy to redirect FROM form_of TO the lemma.
    //
    // At worst, just move this back to process_alt_forms, to not modify the senses.
    for alt_form in &sense.alt_of {
        // If there was any alt_of, we consider this handled, even if we don't add it to forms
        handled = true;

        // [en-en] Ignore variations of little value.
        // misspellings: https://en.wiktionary.org/wiki/peninsular#English
        // misconstruction: https://en.wiktionary.org/wiki/attributative#English
        // non-standard: https://en.wiktionary.org/wiki/%27cept
        // pronunciation-spelling: https://en.wiktionary.org/wiki/Espanish#English
        // obsolete: https://en.wiktionary.org/wiki/enhabit#English
        // abbreviation: https://en.wiktionary.org/wiki/pp#English
        //
        // NOTE: At some point, this begs the question as to why even add alt_of...
        if sense.tags.iter().any(|tag| {
            tag == "misspelling"
                || tag == "misconstruction"
                || tag == "nonstandard"
                || tag == "pronunciation-spelling"
                || tag == "obsolete"
                || tag == "abbreviation"
        }) {
            continue;
        }

        // Some defective entries may not contain the "alt-of" tag. See [de-de] caritativ
        let mut sense_tags = sense.tags.clone();
        if !sense_tags.iter().any(|tag| tag == "alt-of") {
            sense_tags.push("alt-of".to_string());
        }

        irs.insert_form(
            &entry.word,
            &alt_form.word,
            &entry.pos,
            FormSource::AltOfSense,
            sense_tags.clone(),
        );
    }
    handled
}

static JA_SEE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^「?([\p{Hiragana}\p{Katakana}\p{Han}ー]+?)」?\s?を?参照。?$").unwrap()
});
static JA_KANJI_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^「?([\p{Hiragana}\p{Katakana}\p{Han}ー]+?)」?\s?の漢字表記。?$").unwrap()
});

// At least the 漢字表記 shouldn't be needed if they end up fixing their templates,
// but they take ages to fix them, and there's no guarantee that they will fix everything.
//
/// Detects Japanese "see" redirections — scuffed pages that reference another word via 参照
/// instead of using the proper {{wago}} template.
///
/// Example pages:
/// - <https://ja.wiktionary.org/wiki/減らす> (correct page)
/// - <https://ja.wiktionary.org/wiki/好み#Japanese> (このみの漢字表記。)
/// - <https://ja.wiktionary.org/wiki/勢い#日本語> (いきおい　参照)
/// - <https://ja.wiktionary.org/wiki/諄い> (くどいを参照。)
/// - <https://ja.wiktionary.org/wiki/予言> (「かねごと」参照。)
fn handle_see_sense(edition: Edition, entry: &WordEntry, sense: &Sense, irs: &mut Tidy) -> bool {
    let Some(gloss) = (matches!(edition, Edition::Ja))
        .then(|| sense.glosses.first())
        .flatten()
    else {
        return false;
    };
    let Some(target) = JA_SEE_RE
        .captures(gloss)
        .or_else(|| JA_KANJI_RE.captures(gloss))
        .and_then(|caps| caps.get(1))
    else {
        // if gloss.contains("参照") || gloss.contains("漢字表記") {
        //     tracing::warn!("unmatched see/kanji gloss: {:?}", gloss);
        // }
        return false;
    };
    irs.insert_form(
        target.as_str(),
        &entry.word,
        &entry.pos,
        FormSource::Inflection,
        vec![format!("redirected from {}", entry.word)],
    );
    true
}

pub fn normalize_orthography(source: Lang, word: &str) -> String {
    const ARABIC_DIACRITICS: [char; 16] = [
        '\u{0618}', '\u{0619}', '\u{061A}', '\u{064B}', '\u{064C}', '\u{064D}', '\u{064E}',
        '\u{064F}', '\u{0650}', '\u{0651}', '\u{0652}', '\u{0653}', '\u{0654}', '\u{0655}',
        '\u{0656}', '\u{0670}',
    ];
    match source {
        Lang::Ar | Lang::Fa => word
            .chars()
            .filter(|c| !ARABIC_DIACRITICS.contains(c))
            .collect(),
        Lang::La | Lang::Ang | Lang::Sga | Lang::Grc | Lang::Ro | Lang::Id => word
            .nfd()
            .filter(|c| !('\u{0300}'..='\u{036F}').contains(c))
            .nfc()
            .collect(),
        Lang::Tl => word
            .nfd()
            .filter(|c| !('\u{0300}'..='\u{036F}').contains(c) && *c != '-' && *c != '\'')
            .nfc()
            .collect(),
        Lang::Sh => {
            let mut last_base: Option<char> = None;
            let filtered = word.nfd().filter(|&c| {
                if ('\u{0300}'..='\u{036F}').contains(&c) {
                    !matches!(
                        last_base,
                        Some('a' | 'e' | 'i' | 'o' | 'u' | 'r' | 'A' | 'E' | 'I' | 'O' | 'U' | 'R')
                    )
                } else {
                    last_base = Some(c);
                    true
                }
            });
            filtered.nfc().collect()
        }
        Lang::Uk | Lang::Ru => word.replace('\u{0301}', ""),
        _ => word.to_string(),
    }
}
