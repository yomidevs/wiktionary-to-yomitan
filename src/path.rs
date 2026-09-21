//! Helper module to manage paths.

use std::{
    fmt, fs,
    path::{Path, PathBuf},
};

use crate::{
    cli::{DictName, LangSpecs, Options},
    dict::WriterFormat,
    lang::{Edition, EditionSpec, Lang},
};

// This abstraction leaks.
//
/// Enum used by [`PathManager`] to manage filetree operations.
#[derive(Debug, Clone, Copy)]
pub enum DictionaryType {
    Main,
    Glossary,
    GlossaryExtended,
    Ipa,
    IpaMerged,
}

/// Used only for the temporary files folder (`dir_temp`).
impl fmt::Display for DictionaryType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Main => "main",
            Self::Glossary => "glossary",
            Self::GlossaryExtended => "glossary-ext",
            Self::Ipa => "ipa",
            Self::IpaMerged => "ipa-merged",
        })
    }
}

/// Kind of the path. Filtered (by language) for tests. Unfiltered for Kaikki downloads.
//
// this could also be used to ingest some other shape of the data (like rkyv Archive)
#[derive(Debug, PartialEq, Eq)]
pub enum PathKind {
    /// Path to a filtered jsonl. For tests only.
    Filtered,
    /// Path to a unfiltered jsonl
    Unfiltered,
}

#[derive(Debug, PartialEq, Eq)]
struct DatasetPath {
    kind: PathKind,
    path: PathBuf,
}

/// A Vec of paths with their respective [`PathKind`].
#[derive(Debug, PartialEq, Eq)]
pub struct DatasetPaths {
    inner: Vec<DatasetPath>,
}

impl DatasetPaths {
    pub fn of_kind(&self, kinds: &[PathKind]) -> Vec<PathBuf> {
        self.inner
            .iter()
            .filter_map(|p| {
                if kinds.contains(&p.kind) {
                    Some(p.path.clone())
                } else {
                    None
                }
            })
            .collect()
    }
}

fn dataset_raw_unfiltered(edition: Edition, root: PathBuf) -> DatasetPath {
    DatasetPath {
        kind: PathKind::Unfiltered,
        path: root.join(format!("{edition}-extract.jsonl")),
    }
}

fn dataset_raw_filtered(edition: Edition, lang: Lang, root: PathBuf) -> DatasetPath {
    DatasetPath {
        kind: PathKind::Filtered,
        path: root.join(format!("{lang}-{edition}-extract.jsonl")),
    }
}

impl DatasetPaths {
    pub fn new(edition: Edition, lang: Option<Lang>, root: PathBuf) -> Self {
        Self {
            inner: match lang {
                Some(lang) => vec![
                    dataset_raw_filtered(edition, lang, root.clone()),
                    dataset_raw_unfiltered(edition, root),
                ],
                None => vec![dataset_raw_unfiltered(edition, root)],
            },
        }
    }
}

/// Helper struct to manage paths.
///
/// TODO: rename to context/params
#[derive(Debug, Clone)]
pub struct PathManager {
    pub dict_ty: DictionaryType,
    pub dict_name: DictName,
    pub langs: LangSpecs,
    pub opts: Options,
}

impl PathManager {
    pub const fn set_edition(&mut self, edition: EditionSpec) {
        self.langs.edition = edition;
    }
    pub const fn set_source(&mut self, source: Lang) {
        self.langs.source = source;
    }
    pub const fn set_target(&mut self, target: Lang) {
        self.langs.target = target;
    }

    pub const fn langs(&self) -> (EditionSpec, Lang, Lang) {
        let LangSpecs {
            edition,
            source,
            target,
        } = self.langs;
        (edition, source, target)
    }

    const fn root_dir(&self) -> &PathBuf {
        &self.opts.root_dir
    }

    /// Example: `data/kaikki`
    pub fn dir_kaik(&self) -> PathBuf {
        self.root_dir().join("kaikki")
    }
    /// Directory for all dictionaries.
    ///
    /// Example: `data/dict`
    pub fn dir_dicts(&self) -> PathBuf {
        self.root_dir().join("dict")
    }
    /// Example: `data/dict/el/el`
    fn dir_dict(&self) -> PathBuf {
        self.dir_dicts().join(match self.dict_ty {
            DictionaryType::IpaMerged => format!("{}/{}", self.langs.edition, self.langs.target),
            _ => format!("{}/{}", self.langs.source, self.langs.target),
        })
    }
    /// Depends on the type of dictionary being made.
    ///
    /// Example: `data/dict/el/el/main/`
    /// Example: `data/dict/el/el/glossary/`
    //
    // TODO: make this depend on WriterFromat?
    fn dir_stage(&self) -> PathBuf {
        let dir_stage = self.dir_dict().join(self.dict_ty.to_string());

        match self.dict_ty {
            // For GlossaryExtended, we need to add the edition to disambiguate intermediary
            // files (the dictionary zip names differ anyway). That is (dir_stage between ``):
            // `data/dict/el/en/glossary-ext/de/`tidy/tidy.jsonl
            // `data/dict/el/en/glossary-ext/fr/`tidy/tidy.jsonl
            DictionaryType::GlossaryExtended => dir_stage.join(self.langs.edition.to_string()),
            _ => dir_stage,
        }
    }
    /// Example: `data/dict/el/el/main/path`
    /// Example: `data/dict/el/el/glossary/path`
    pub fn dir_in_stage<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        self.dir_stage().join(path)
    }
    /// Example: `data/dict/el/el/main/tidy`
    /// Example: `data/dict/el/el/glossary/tidy`
    pub fn dir_tidy(&self) -> PathBuf {
        self.dir_in_stage("tidy")
    }

    pub fn setup_dirs(&self) -> anyhow::Result<()> {
        fs::create_dir_all(self.dir_kaik())?;
        fs::create_dir_all(self.dir_dict())?;

        // TODO: test this
        if matches!(self.opts.format, WriterFormat::Ir) {
            fs::create_dir_all(self.dir_tidy())?;
            fs::create_dir_all(self.dir_temp_dict())?;
        }

        Ok(())
    }

    pub fn dataset_paths(&self, edition: Edition, lang: Option<Lang>) -> DatasetPaths {
        DatasetPaths::new(edition, lang, self.dir_kaik())
    }

    /// `data/dict/source/target/temp/tidy/source-target-lemmas.json`
    ///
    /// Example: `data/dict/el/el/temp/tidy/el-el-lemmas.json`
    pub fn path_lemmas(&self) -> PathBuf {
        self.dir_tidy().join(format!(
            "{}-{}-lemmas.json",
            self.langs.source, self.langs.target
        ))
    }

    /// `data/dict/source/target/temp/tidy/source-target-forms.json`
    ///
    /// Example: `data/dict/el/el/temp/tidy/el-el-forms.json`
    pub fn path_forms(&self) -> PathBuf {
        self.dir_tidy().join(format!(
            "{}-{}-forms.json",
            self.langs.source, self.langs.target
        ))
    }

    /// Temporary working directory path used before zipping the dictionary.
    ///
    /// Example: `data/dict/el/el/temp/dict`
    pub fn dir_temp_dict(&self) -> PathBuf {
        self.dir_stage().join("dict")
    }

    // Should not go here, but since it uses dict_ty...
    // It exists so the dictionary index is in sync with PathManager::path_dict
    //
    /// Depends on the dictionary type (main, glossary etc.)
    ///
    /// Example: `dictionary_name-el-en`
    /// Example: `dictionary_name-el-en-gloss`
    pub fn dict_name_expanded(&self) -> String {
        let dict_name = &self.dict_name;
        let LangSpecs {
            edition,
            source,
            target,
        } = self.langs;

        use DictionaryType::*;
        let mut expanded = match self.dict_ty {
            Main => format!("{dict_name}-{source}-{target}"),
            Glossary => format!("{dict_name}-{source}-{target}-gloss"),
            GlossaryExtended => format!("{dict_name}-{edition}-{source}-{target}-gloss"),
            Ipa => format!("{dict_name}-{source}-{target}-ipa"),
            IpaMerged => format!("{dict_name}-{target}-ipa"),
        };

        if self.opts.experimental {
            expanded.push_str("-exp");
        }

        expanded
    }

    /// Depends on the dictionary type (main, glossary etc.)
    ///
    /// Example: `data/dict/el/en/dictionary_name-el-en.zip`
    /// Example: `data/dict/el/en/dictionary_name-el-en-gloss.zip`
    pub fn path_dict(&self) -> PathBuf {
        self.dir_dict()
            .join(format!("{}.zip", self.dict_name_expanded()))
    }

    /// Example: `data/dict/el/el/temp/diagnostics`
    pub fn dir_diagnostics(&self) -> PathBuf {
        self.dir_stage().join("diagnostics")
    }
}
