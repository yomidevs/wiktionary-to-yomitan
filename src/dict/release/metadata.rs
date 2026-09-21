//! Dictionary release metadata.
//!
//! Scans the release `dict/` folder and produces a `release_metadata.json` summarizing
//! the size of each dictionary type, source language, and target language.
//!
//! Metadata is written to the `docs/` folder to be used in the downloads page.
//!
//! The only timing is the total time of the release: per dictionary timings mostly
//! pollute the diff with variations that are due to threading.

use std::{collections::BTreeMap, path::Path, time::Duration};

use anyhow::Result;
use serde::ser::SerializeStruct;

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

/// Which dictionary wrote `<source>/<target>/<stem>.zip`.
fn classify_dict(source: &str, target: &str, stem: &str) -> &'static str {
    if source == "all" {
        return "ipa-merged";
    }

    if stem.ends_with("-ipa") {
        return "ipa";
    }

    let Some(body) = stem.strip_suffix("-gloss") else {
        return "main";
    };

    // glossary is  `<name>-<source>-<target>` 
    // glossary-ext `<name>-<edition>-<source>-<target>`.
    let head = body
        .strip_suffix(&format!("-{source}-{target}"))
        .unwrap_or(body);
    if head.contains('-') {
        "glossary-ext"
    } else {
        "glossary"
    }
}

fn scan_and_group(root_dir: &Path) -> Result<Metadata> {
    let mut meta = Metadata::default();

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

        let stem = filename.strip_suffix(".zip").unwrap_or(&filename);
        let dict_type = classify_dict(&source, &target, stem);
        let size = path.metadata()?.len();

        let type_entry = meta.dicts.entry(dict_type.to_string()).or_default();
        let src = type_entry.sources.entry(source.clone()).or_default();

        let target_info = TargetInfo { size };

        // Insert or update target info
        src.targets.insert(target.clone(), target_info);
        src.size += size;
        src.count += 1;

        type_entry.size += size;
        type_entry.count += 1;

        meta.size += size;
        meta.count += 1;
    }

    Ok(meta)
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

/// Write the metadata of the release at `root_dir`.
///
/// `time` is the total time of the release.
pub fn write_dict_metadata(root_dir: &Path, editions: &[Edition], time: Duration) -> Result<()> {
    let dict_dir = root_dir.join("dict");
    let mut metadata = scan_and_group(&dict_dir)?;
    metadata.time = time;
    add_db_metadata(root_dir, editions, &mut metadata)?;
    let json = serde_json::to_string_pretty(&metadata)?;
    let out_path = Path::new("docs/release_metadata.json");
    std::fs::write(out_path, &json)?;
    println!("[meta] Dict metadata written to {}", out_path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify() {
        assert_eq!(classify_dict("el", "en", "wty-el-en"), "main");
        assert_eq!(classify_dict("el", "en", "wty-el-en-gloss"), "glossary");
        assert_eq!(
            classify_dict("el", "en", "wty-all-el-en-gloss"),
            "glossary-ext"
        );
        assert_eq!(
            classify_dict("el", "en", "wty-de-el-en-gloss"),
            "glossary-ext"
        );
        assert_eq!(classify_dict("el", "en", "wty-el-en-ipa"), "ipa");
        assert_eq!(classify_dict("all", "en", "wty-en-ipa"), "ipa-merged");

        // isos containing a dash
        assert_eq!(classify_dict("gem-pro", "en", "wty-gem-pro-en-ipa"), "ipa");
    }
}
