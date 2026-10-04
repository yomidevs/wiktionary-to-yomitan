//! Dictionary release metadata.
//!
//! Produces a `release_metadata_<format>.json` per format from the dictionaries the release
//! built, summarizing the size of each dictionary type, source language, and target language.
//!
//! Metadata is written to the `docs/` folder to be used in the downloads page.
//!
//! The only timing is the total time of the release: per dictionary timings mostly
//! pollute the diff with variations that are due to threading.

use std::{collections::BTreeMap, path::Path, time::Duration};

use anyhow::Result;
use serde::ser::SerializeStruct;

use super::Built;
use crate::dict::WriterFormat;
use crate::lang::Edition;
use crate::utils::{human_size, human_time};

#[derive(Debug, Default)]
struct TargetInfo {
    size: u64,
}

#[derive(Debug, Default)]
struct SourceInfo {
    size: u64,
    count: u64,
    targets: BTreeMap<String, TargetInfo>,
}

#[derive(Debug, Default)]
struct TypeInfo {
    size: u64,
    count: u64,
    sources: BTreeMap<String, SourceInfo>,
}

type DictInfo = BTreeMap<String, TypeInfo>;

#[derive(Debug, Default)]
struct DbInfo {
    size: u64,
}

#[derive(Debug, Default)]
struct Metadata {
    time: Duration,
    size: u64,
    count: u64,
    db: BTreeMap<String, DbInfo>,
    dicts: DictInfo,
}

impl serde::Serialize for TargetInfo {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&human_size(self.size as f64))
    }
}

impl serde::Serialize for SourceInfo {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut state = s.serialize_struct("SourceInfo", 3)?;
        state.serialize_field("size", &human_size(self.size as f64))?;
        state.serialize_field("count", &self.count)?;
        state.serialize_field("targets", &self.targets)?;
        state.end()
    }
}

impl serde::Serialize for TypeInfo {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut state = s.serialize_struct("TypeInfo", 3)?;
        state.serialize_field("size", &human_size(self.size as f64))?;
        state.serialize_field("count", &self.count)?;
        state.serialize_field("sources", &self.sources)?;
        state.end()
    }
}

impl serde::Serialize for Metadata {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut state = s.serialize_struct("Metadata", 5)?;
        state.serialize_field("time", &human_time(self.time))?;
        state.serialize_field("size", &human_size(self.size as f64))?;
        state.serialize_field("count", &self.count)?;
        state.serialize_field("db", &self.db)?;
        state.serialize_field("dicts", &self.dicts)?;
        state.end()
    }
}

impl serde::Serialize for DbInfo {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut state = s.serialize_struct("DbInfo", 1)?;
        state.serialize_field("size", &human_size(self.size as f64))?;
        state.end()
    }
}

fn group(built: &[Built], format: WriterFormat) -> Metadata {
    let mut meta = Metadata::default();

    for dict in built {
        let Some(&(_, size)) = dict.sizes.iter().find(|(f, _)| *f == format) else {
            continue;
        };

        let type_entry = meta.dicts.entry(dict.ty.to_string()).or_default();
        let src = type_entry.sources.entry(dict.source.clone()).or_default();

        src.targets.insert(dict.target.clone(), TargetInfo { size });
        src.size += size;
        src.count += 1;

        type_entry.size += size;
        type_entry.count += 1;

        meta.size += size;
        meta.count += 1;
    }

    meta
}

fn add_db_metadata(root_dir: &Path, editions: &[Edition], metadata: &mut Metadata) -> Result<()> {
    let db_dir = root_dir.join("db");

    for edition in editions {
        let db_path = db_dir.join(format!("wiktextract_{edition}.db"));
        let size = db_path.metadata()?.len();
        metadata.db.insert(edition.to_string(), DbInfo { size });
    }

    Ok(())
}

/// Write the metadata of the release at `root_dir`, one file per format built.
///
/// `time` is the total time of the release.
pub fn write_dict_metadata(
    root_dir: &Path,
    editions: &[Edition],
    built: &[Built],
    time: Duration,
) -> Result<()> {
    let mut formats = Vec::new();
    for (format, _) in built.iter().flat_map(|dict| &dict.sizes) {
        if !formats.contains(format) {
            formats.push(*format);
        }
    }

    for format in formats {
        let mut metadata = group(built, format);
        metadata.time = time;
        add_db_metadata(root_dir, editions, &mut metadata)?;

        let path = format!("docs/release_metadata_{format}.json");
        std::fs::write(&path, serde_json::to_string_pretty(&metadata)?)?;
        println!("[meta] Dict metadata written to {path}");
    }

    Ok(())
}
