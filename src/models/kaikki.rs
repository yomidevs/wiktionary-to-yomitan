//! Wiktextract / kaikki data model.
//!
//! Non-en JSON schemas:
//! <https://tatuylonen.github.io/wiktextract>
//! <https://tatuylonen.github.io/wiktextract/compare_schemas.html>
//!
//! There is no EN JSON schema but there are some approximations:
//! <https://kaikki.org/dictionary/errors/mapping/index.html>
//! <https://github.com/tatuylonen/wiktextract/blob/master/src/wiktextract/extractor/en/type_utils.py>
//!
//! Example (el):
//! <https://github.com/tatuylonen/wiktextract/blob/master/src/wiktextract/extractor/el/models.py>

use serde::{Deserialize, Serialize};

use crate::tags::{BLACKLISTED_FORM_TAGS, IDENTITY_FORM_TAGS, ScriptTag};

// In case we ever decide to narrow them
pub type Tag = String;

#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(default)]
#[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct WordEntry {
    pub word: String,
    pub pos: String,

    pub lang_code: String,

    pub head_templates: Vec<HeadTemplate>,

    // Not pub because unstable: use the getter method
    etymology_text: String, // En, El editions still use this
    etymology_texts: Vec<String>,

    pub sounds: Vec<Sound>,

    pub senses: Vec<Sense>,

    pub tags: Vec<Tag>,
    pub topics: Vec<Tag>,

    pub forms: Vec<Form>,
    pub form_of: Vec<AltForm>,
    pub alt_of: Vec<AltForm>,

    pub synonyms: Vec<Synonym>,

    pub translations: Vec<Translation>, // used in glossary
}

// To be avoided as much as possible: sort of internal field.
#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(default)]
#[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct HeadTemplate {
    pub expansion: String,
}

#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(default)]
#[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct Sound {
    pub ipa: String,
    pub tags: Vec<Tag>,
    pub note: String,
    pub zh_pron: String,
    // pub other: String, // [ja]
}

#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(default)]
#[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct Sense {
    // Glosses are usually a one string vector, but when there's more, it follows:
    // ["Gloss supercategory", "Specific gloss.", "More specific...", etc.]
    // cf. https://en.wiktionary.org/wiki/pflegen
    pub glosses: Vec<String>,
    pub examples: Vec<Example>,
    pub form_of: Vec<AltForm>,
    pub alt_of: Vec<AltForm>,
    pub tags: Vec<Tag>,
    pub topics: Vec<Tag>,
}

pub type Offset = (usize, usize);

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
#[serde(default)]
#[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct Example {
    pub text: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub translation: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(rename = "ref")]
    pub reference: String, // Reference of a quotation example
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub bold_text_offsets: Vec<Offset>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub bold_translation_offsets: Vec<Offset>, // [en]
    #[serde(
        rename(deserialize = "tags"),
        deserialize_with = "ScriptTag::deserialize_kaikki_tags",
        skip_serializing_if = "Option::is_none"
    )]
    pub lang: Option<ScriptTag>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(default)]
#[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct AltForm {
    pub word: String,
}

#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(default)]
#[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct Form {
    pub form: String,
    pub tags: Vec<Tag>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub ruby: Vec<(String, String)>, // [ja] (kanji, hiragana)
}

// * We don't extract Synonyms in Senses, even though some editions use them.
// * We also don't disambiguate to which Sense matches a certain Synonym because
// the wiktextract field sense_index is sometimes an int / sometimes a String.
// TODO: report / fix the above in wiktextract...
#[derive(Debug, Deserialize, Serialize, Default, Clone)]
#[serde(default)]
#[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct Synonym {
    pub word: String,
    // Unused for the moment
    // #[serde(skip_serializing_if = "Vec::is_empty")]
    // pub tags: Vec<Tag>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(default)]
#[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct Translation {
    pub lang_code: String,
    pub word: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub sense: String,
    // These two are used to get the reading for GlossaryExtended
    #[serde(skip_serializing_if = "String::is_empty")]
    pub roman: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub alt: String,
}

// WordEntry impls
//
// These should cover general functions usable for any dictionary and even for external users of
// the WordEntry type.
impl WordEntry {
    // https://github.com/tatuylonen/wiktextract/pull/1489
    pub fn is_participle(&self) -> bool {
        self.pos == "verb" && self.tags.iter().any(|t| t == "participle")
    }

    /// Return all non-empty forms that contain all given tags.
    fn tagged_forms<'a>(&'a self, tags: &[&str]) -> impl Iterator<Item = &'a Form> {
        self.forms.iter().filter(|form| {
            !form.form.is_empty() && tags.iter().all(|tag| form.tags.iter().any(|t| t == tag))
        })
    }

    /// Return the first non-empty form with the `canonical` tag.
    pub fn canonical_form(&self) -> Option<&Form> {
        self.tagged_forms(&["canonical"]).next()
    }

    /// Return the first non-empty form with the `romanization` tag.
    pub fn romanization_form(&self) -> Option<&Form> {
        self.tagged_forms(&["romanization"]).next()
    }

    /// Return the first non-empty form with the `transliteration` tag.
    pub fn transliteration_form(&self) -> Option<&Form> {
        self.tagged_forms(&["transliteration"]).next()
    }

    /// Return the first `sound.zh_pron` with the `Pinyin` tag.
    pub fn pinyin(&self) -> Option<&str> {
        self.sounds.iter().find_map(|sound| {
            if sound.tags.iter().any(|t| t == "Pinyin") {
                Some(sound.zh_pron.as_ref())
            } else {
                None
            }
        })
    }

    /// Check if a `entry` contains no glosses.
    ///
    /// There is a "no-gloss" tag but it is not always in the same place and therefore unreliable.
    pub fn contains_no_gloss(&self) -> bool {
        self.senses.iter().all(|sense| sense.glosses.is_empty())
    }

    pub fn non_trivial_forms(&self) -> impl Iterator<Item = &Form> {
        self.forms.iter().filter(move |form| {
            if form.form.is_empty() || form.form == self.word {
                return false;
            }

            // blacklisted forms (happens at least in English)
            // * "-" usually denotes an empty cell in some table in most editions.
            // * hyphen-prefixed words are more likely than not garbage from inflections.
            // We deal with both at the same time.
            if form.form.starts_with(['-', '‑']) {
                return false;
            }

            // blacklisted tags (happens at least in Russian: romanization)
            let is_blacklisted = form
                .tags
                .iter()
                .any(|tag| BLACKLISTED_FORM_TAGS.contains(&tag.as_str()));
            let is_identity = form
                .tags
                .iter()
                .all(|tag| IDENTITY_FORM_TAGS.contains(&tag.as_str()));
            if is_blacklisted || is_identity {
                return false;
            }

            true
        })
    }

    // Rare, but a translation can have an empty word.
    pub fn non_trivial_translations(&self) -> impl Iterator<Item = &Translation> {
        self.translations
            .iter()
            .filter(move |translation| !translation.word.is_empty())
    }

    pub fn etymology_texts(&self) -> Option<Vec<&str>> {
        if !self.etymology_texts.is_empty() {
            Some(self.etymology_texts.iter().map(String::as_ref).collect())
        } else if !self.etymology_text.is_empty() {
            Some(vec![&self.etymology_text])
        } else {
            None
        }
    }
}
