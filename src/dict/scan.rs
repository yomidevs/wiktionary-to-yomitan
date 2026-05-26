//! Scan the Kaikki jsonlines for diagnostics.
//!
//! Analyzes tags, POS, and localization coverage across dictionary entries.

#![allow(unused)]

use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader, BufWriter},
    path::PathBuf,
};

use anyhow::Result;
use serde::{Serialize, Serializer, ser::SerializeMap};

use crate::{
    cli::{DictName, MainLangs, Options},
    dict::{LangCodeProbe, main::get_reading},
    download::find_or_download_jsonl,
    lang::{Edition, Lang},
    models::kaikki::WordEntry,
    path::{DictionaryType, PathManager},
    tags::{find_tag_in_bank, localize_tag},
    utils::link_wiktionary,
};

/// Any tag with this count or less are not serialized. They are too rare to care.
const MIN_COUNT_TO_SHOW: usize = 10;
const N_LINKS_TO_SHOW: usize = 1;

const TAGS_TO_IGNORE: [&str; 3] = ["unknown", "form-of", "alt-of"];

#[derive(Default, Serialize)]
struct TagDiagnostics {
    // total: TagCounter,
    not_found: TagCounterWithLink,
    not_localized: TagCounterWithLink,
}

impl TagDiagnostics {
    fn process(&mut self, edition: Edition, source: Lang, target: Lang, tag: String, word: &str) {
        if TAGS_TO_IGNORE.contains(&tag.as_str()) {
            return;
        }

        self.process_simple(tag.clone());

        match find_tag_in_bank(&tag) {
            // No localizations for English: go a bit faster
            Some(_) if edition == Edition::En => (),
            Some(tag_info) => match localize_tag(target, &tag_info.short_tag) {
                Some(_) => (),
                // TODO: maybe don't log not localized tags if the short form was an emoji,
                // since those are supposed to be language-agnostic
                None => self.not_localized.increment(tag, edition, source, word),
            },
            None => self.not_found.increment(tag, edition, source, word),
        }
    }

    fn process_simple(&mut self, tag: String) {
        // self.total.increment(tag);
    }
}

/// Counter helper to sort descendingly by value when serialized
#[derive(Default)]
struct TagCounter(HashMap<String, usize>);

impl TagCounter {
    fn increment(&mut self, key: String) {
        *self.0.entry(key).or_insert(0) += 1;
    }
}

impl Serialize for TagCounter {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut items: Vec<_> = self.0.iter().collect();
        items.sort_by(|a, b| b.1.cmp(a.1));
        let mut map = serializer.serialize_map(Some(items.len()))?;
        for (k, v) in items {
            if *v > MIN_COUNT_TO_SHOW {
                map.serialize_entry(k, v)?;
            }
        }
        map.end()
    }
}

#[derive(Serialize)]
struct TagInfo {
    count: usize,
    links: Vec<String>,
}

/// Counter with link to first occurrence
#[derive(Default)]
struct TagCounterWithLink(HashMap<String, TagInfo>);

impl TagCounterWithLink {
    fn increment(&mut self, key: String, edition: Edition, source: Lang, word: &str) {
        self.0
            .entry(key)
            .and_modify(|e| {
                e.count += 1;
                if e.links.len() < N_LINKS_TO_SHOW {
                    e.links.push(link_wiktionary(edition, source, word));
                }
            })
            .or_insert_with(|| TagInfo {
                count: 1,
                links: vec![link_wiktionary(edition, source, word)],
            });
    }
}

impl Serialize for TagCounterWithLink {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut items: Vec<_> = self.0.iter().collect();
        items.sort_by(|a, b| b.1.count.cmp(&a.1.count));
        let mut map = serializer.serialize_map(Some(items.len()))?;
        for (k, v) in items {
            if v.count > MIN_COUNT_TO_SHOW {
                map.serialize_entry(k, v)?;
            }
        }
        map.end()
    }
}

/// Check if the word has at least some Kanji.
///
/// This is used to exclude kana-only words from being labelled as having no readings.
fn has_kanji(word: &str) -> bool {
    word.chars().any(|c| {
        matches!(c, '\u{4E00}'..='\u{9FFF}'
            | '\u{3400}'..='\u{4DBF}'
            | '\u{F900}'..='\u{FAFF}'
            | '\u{20000}'..='\u{2A6DF}'
            | '\u{2A700}'..='\u{2CEAF}'
            | '\u{2CEB0}'..='\u{2EBEF}'
        )
    })
}

#[derive(Default, Serialize)]
struct Diagnostics {
    words: i32,
    words_without_reading: i32,
    pos: TagDiagnostics,
    tags_top: TagDiagnostics,
    tags_inner: TagDiagnostics,
    tags_forms: TagDiagnostics,
}

impl Diagnostics {
    fn new() -> Self {
        Self::default()
    }

    fn process_entry(&mut self, edition: Edition, source: Lang, entry: WordEntry) {
        self.words += 1;

        if get_reading(edition, source, &entry).is_none() {
            match (edition, source) {
                (Edition::Ja, Lang::Ja) if !has_kanji(&entry.word) => (),
                _ => self.words_without_reading += 1,
            }
        }

        let target: Lang = edition.into();
        let word = &entry.word;

        self.pos.process(edition, source, target, entry.pos, word);

        for tag in entry.tags {
            self.tags_top.process(edition, source, target, tag, word);
        }

        for sense in entry.senses {
            for tag in sense.tags {
                self.tags_inner.process(edition, source, target, tag, word);
            }
        }

        for form in entry.forms {
            for tag in form.tags {
                // These are never localized
                self.tags_forms.process_simple(tag);
            }
        }
    }

    fn write(&self, output_path: &str) -> Result<()> {
        let file = File::create(output_path)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, self)?;
        tracing::info!("Diagnostics written @ {}", output_path);
        Ok(())
    }
}

fn setup_args(args: MainLangs) -> Result<(Edition, Lang, PathBuf)> {
    let edition = args.target;
    let source = args.source;

    // We just hack a pm to point to the jsonl folder
    let pm = PathManager {
        dict_name: DictName::default(),
        langs: args.try_into()?,
        dict_ty: DictionaryType::Main,
        opts: Options {
            root_dir: "data".into(),
            ..Default::default()
        },
    };

    let path_jsonl = find_or_download_jsonl(edition, Some(source), &pm)?;
    Ok((edition, source, path_jsonl))
}

/// Scan the Kaikki jsonlines for diagnostics.
pub fn scan(args: MainLangs) -> Result<()> {
    let (edition, source, path_jsonl) = setup_args(args)?;

    tracing::warn!("Ignoring tags with count <= {MIN_COUNT_TO_SHOW}");
    tracing::warn!("Showing at most {N_LINKS_TO_SHOW} link(s)");

    let capacity = 256 * (1 << 10); // default is 8 * (1 << 10) := 8KB
    let mut line = Vec::with_capacity(1 << 10);
    let reader_file = File::open(&path_jsonl)?;
    let mut reader = BufReader::with_capacity(capacity, reader_file);
    let mut diagnostics = Diagnostics::new();

    loop {
        line.clear();
        if reader.read_until(b'\n', &mut line)? == 0 {
            break; // EOF
        }

        if LangCodeProbe::should_skip(&line, source)? {
            continue;
        }

        let entry: WordEntry = serde_json::from_slice(&line)?;
        diagnostics.process_entry(edition, source, entry);
    }

    let output_path = format!("diagnostics_{source}_{edition}.json");
    diagnostics.write(&output_path)?;

    Ok(())
}
