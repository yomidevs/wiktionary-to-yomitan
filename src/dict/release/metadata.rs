//! Dictionary release metadata.
//!
//! Scans the release `dict/` folder and produces a `release_metadata.json` summarizing
//! the size of each dictionary type, source language, and target language.
//!
//! Metadata is written to the `docs/` folder to be used in the downloads page.

use std::{collections::BTreeMap, path::Path};

use anyhow::Result;
use serde::ser::SerializeStruct;

use crate::dict::release::TimingStats;
use crate::utils::{human_size, human_time};

// There is not time for TargetInfo because most of the time is instant and it
// pollutes the diff with variations that are mainly due to threading.
#[derive(Debug, Default)]
struct TargetInfo {
    size: u64,
}

#[derive(Debug, Default)]
struct SourceInfo {
    size: u64,
    count: u64,
    time: u128,
    targets: BTreeMap<String, TargetInfo>,
}

#[derive(Debug, Default)]
struct TypeInfo {
    size: u64,
    count: u64,
    time: u128,
    sources: BTreeMap<String, SourceInfo>,
}

type DictInfo = BTreeMap<String, TypeInfo>;

#[derive(Debug, Default)]
struct DbInfo {
    size: u64,
    time: u128,
}

#[derive(Debug, Default)]
struct Metadata {
    size: u64,
    count: u64,
    time: u128,
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
        let mut state = s.serialize_struct("SourceInfo", 4)?;
        state.serialize_field("size", &human_size(self.size as f64))?;
        state.serialize_field("count", &self.count)?;
        state.serialize_field("time", &human_time(self.time))?;
        state.serialize_field("targets", &self.targets)?;
        state.end()
    }
}

impl serde::Serialize for TypeInfo {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut state = s.serialize_struct("TypeInfo", 4)?;
        state.serialize_field("size", &human_size(self.size as f64))?;
        state.serialize_field("count", &self.count)?;
        state.serialize_field("time", &human_time(self.time))?;
        state.serialize_field("sources", &self.sources)?;
        state.end()
    }
}

impl serde::Serialize for Metadata {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut state = s.serialize_struct("Metadata", 5)?;
        state.serialize_field("size", &human_size(self.size as f64))?;
        state.serialize_field("count", &self.count)?;
        state.serialize_field("time", &human_time(self.time))?;
        state.serialize_field("db", &self.db)?;
        state.serialize_field("dicts", &self.dicts)?;
        state.end()
    }
}

impl serde::Serialize for DbInfo {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut state = s.serialize_struct("DbInfo", 2)?;
        state.serialize_field("size", &human_size(self.size as f64))?;
        state.serialize_field("time", &human_time(self.time))?;
        state.end()
    }
}

fn classify_dict(name: &str) -> &str {
    if name.ends_with("-ipa") {
        // wty-afb-en-ipa
        if name.chars().filter(|c| *c == '-').count() == 3 {
            "ipa"
        // wty-afb-ipa
        } else {
            "ipa-merged"
        }
    } else if name.ends_with("-gloss") {
        "glossary"
    } else {
        "main"
    }
}

fn scan_and_group(root_dir: &Path, stats: &TimingStats) -> Result<Metadata> {
    let mut meta = Metadata::default();
    let timings = stats.timings.lock().unwrap();

    for entry in walkdir::WalkDir::new(root_dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|x| x == "zip"))
    {
        let path = entry.path();

        // Expect <root>/<source>/<target>/<dict-name>.zip
        let rel = path.strip_prefix(root_dir)?;
        let parts: Vec<_> = rel.components().collect();
        if parts.len() != 3 {
            continue;
        }

        let source = parts[0].as_os_str().to_string_lossy().into_owned();
        let target = parts[1].as_os_str().to_string_lossy().into_owned();
        let filename = parts[2].as_os_str().to_string_lossy();

        let dict_type = classify_dict(filename.trim_end_matches(".zip"));
        let size = path.metadata()?.len();

        let type_entry = meta.dicts.entry(dict_type.to_string()).or_default();
        let src = type_entry.sources.entry(source.clone()).or_default();

        // TODO: fix this (?)
        let timing_key = match dict_type {
            "ipa-merged" => format!("ipa-merged-{target}"),
            other => {
                format!("{other}-{source}-{target}")
            }
        };
        let time = if let Some(time) = timings.get(&timing_key) {
            time.as_millis()
        } else {
            tracing::warn!("Key {timing_key} was not found");
            0
        };

        let target_info = TargetInfo { size };

        // Insert or update target info
        src.targets.insert(target.clone(), target_info);
        src.size += size;
        src.count += 1;
        src.time += time;

        type_entry.size += size;
        type_entry.count += 1;
        type_entry.time += time;

        meta.size += size;
        meta.count += 1;
        meta.time += time;
    }

    Ok(meta)
}

fn add_db_metadata(root_dir: &Path, db_stats: &TimingStats, metadata: &mut Metadata) -> Result<()> {
    let db_timings = db_stats.timings.lock().unwrap();
    let db_dir = root_dir.join("db");

    for (edition, timing) in db_timings.iter() {
        let db_path = db_dir.join(format!("wiktextract_{edition}.db"));
        let size = db_path.metadata()?.len();
        let time = timing.as_millis();
        let db_info = DbInfo { size, time };
        metadata.db.insert(edition.clone(), db_info);
    }

    Ok(())
}

pub fn write_dict_metadata(
    root_dir: &Path,
    db_stats: &TimingStats,
    stats: &TimingStats,
) -> Result<()> {
    let dict_dir = root_dir.join("dict");
    let mut metadata = scan_and_group(&dict_dir, stats)?;
    add_db_metadata(root_dir, db_stats, &mut metadata)?;
    let json = serde_json::to_string_pretty(&metadata)?;
    let out_path = Path::new("docs/release_metadata.json");
    std::fs::write(out_path, &json)?;
    println!("[meta] Dict metadata written to {}", out_path.display());
    Ok(())
}
