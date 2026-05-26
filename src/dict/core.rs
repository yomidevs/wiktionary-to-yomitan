//! [`Dictionary`] trait and dictionary build pipeline.

use anyhow::{Context, Ok, Result};
use serde::{Deserialize, Serialize};

use std::{
    borrow::Cow,
    fmt,
    fs::File,
    io::{BufRead, BufReader, BufWriter, Write},
    path::PathBuf,
};

use crate::{
    Map,
    cli::{LangSpecs, Options},
    download::find_or_download_jsonl,
    lang::{Edition, Lang},
    models::{kaikki::WordEntry, yomitan::YomitanDict},
    path::PathManager,
};

const CONSOLE_PRINT_INTERVAL: i32 = 10000;

/// Trait for Intermediate representation.
///
/// Used for postprocessing (merge, etc.) and debugging via snapshots.
pub trait Intermediate: Default {
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// How to write `Self::I` to disk.
    fn write(&self, pm: &PathManager) -> Result<PathBuf>;
}

impl<T> Intermediate for Vec<T>
where
    T: Serialize,
{
    fn len(&self) -> usize {
        Self::len(self)
    }

    fn write(&self, pm: &PathManager) -> Result<PathBuf> {
        let writer_path = pm.dir_tidy().join("tidy.jsonl");
        let writer_file = File::create(&writer_path)?;
        let writer = BufWriter::new(&writer_file);
        if pm.opts.pretty {
            serde_json::to_writer_pretty(writer, self)?;
        } else {
            serde_json::to_writer(writer, self)?;
        }
        Ok(writer_path)
    }
}

impl<A, B> Intermediate for Map<A, B>
where
    A: Serialize,
    B: Serialize,
{
    fn len(&self) -> usize {
        Self::len(self)
    }

    fn write(&self, _: &PathManager) -> Result<PathBuf> {
        unimplemented!()
    }
}

/// Trait to abstract the process of making a dictionary.
pub trait Dictionary {
    type A: TryInto<PathManager, Error = anyhow::Error>;
    type I: Intermediate;

    /// Whether we want to quickly probe a jsonline to avoid a full deserialization.
    ///
    /// By default, it probes on source Lang.
    ///
    /// It is only overwritten in glossary extended, and it is pointless when working
    /// with a database.
    fn supports_probe(&self) -> bool {
        true
    }

    /// Whether to completely ignore this entry.
    #[allow(unused_variables)]
    fn skip_if(&self, entry: &WordEntry) -> bool {
        false
    }

    /// How to preprocess a [`WordEntry`]. Everything that mutates `entry` should go here.
    #[allow(unused_variables)]
    fn preprocess(&self, langs: Langs, entry: &mut WordEntry, opts: &Options, irs: &mut Self::I) {}

    /// How to transform a `WordEntry` into intermediate representation.
    ///
    /// Most dictionaries only make *at most one* `Self::I` from a [`WordEntry`].
    // TODO: why not take ownership of entry?
    fn process(&self, langs: Langs, entry: &WordEntry, irs: &mut Self::I);

    /// How to postprocess the intermediate representation.
    ///
    /// This can be implemented to merge entries from different edition, to postprocess tags etc.
    #[allow(unused_variables)]
    fn postprocess(&self, langs: LangSpecs, irs: &mut Self::I) {}

    /// Console message for found irs. It is customized for the main dictionary.
    fn found_ir_message(&self, langs: LangSpecs, irs: &Self::I) {
        tracing::debug!(
            "[{}-{}] Found {} irs",
            langs.source,
            langs.target,
            irs.len()
        );
    }

    /// How to convert `Self::I` into one or more yomitan entries.
    fn to_yomitan(&self, langs: LangSpecs, irs: &Self::I) -> YomitanDict;
}

fn rejected(entry: &WordEntry, opts: &Options) -> bool {
    opts.reject.iter().any(|(k, v)| k.field_value(entry) == v)
        || !opts.filter.iter().all(|(k, v)| k.field_value(entry) == v)
}

/// Unified language configuration. See [`crate::cli::LangSpecs`].
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Langs {
    pub edition: Edition,
    pub source: Lang,
    pub target: Lang,
}

impl Langs {
    pub const fn new(edition: Edition, source: Lang, target: Lang) -> Self {
        Self {
            edition,
            source,
            target,
        }
    }
}

impl fmt::Debug for Langs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Langs")
            .field(&self.edition)
            .field(&self.source)
            .field(&self.target)
            .finish()
    }
}

pub fn iter_datasets(pm: &PathManager) -> impl Iterator<Item = Result<(Edition, PathBuf)>> + '_ {
    let (edition_pm, source_pm, _) = pm.langs();

    edition_pm.variants().into_iter().map(move |edition| {
        let path_jsonl = find_or_download_jsonl(edition, Some(source_pm), pm)?;
        tracing::trace!("edition: {edition}, path: {}", path_jsonl.display());

        Ok((edition, path_jsonl))
    })
}

#[derive(Deserialize)]
#[serde(default)]
pub struct LangCodeProbe<'a> {
    #[serde(borrow)]
    lang_code: Cow<'a, str>,
}

impl LangCodeProbe<'_> {
    pub fn should_skip(line: &[u8], lang: Lang) -> Result<bool> {
        let probe: LangCodeProbe =
            serde_json::from_slice(line).with_context(|| "Error decoding JSON @ probe")?;
        Ok(probe.lang_code != lang.iso())
    }
}

impl Default for LangCodeProbe<'_> {
    fn default() -> Self {
        Self {
            lang_code: Cow::Borrowed(""),
        }
    }
}

/// Make a dictionary from a Kaikki jsonlines.
pub fn make_dict_from_jsonl<D: Dictionary>(dict: D, raw_args: D::A) -> Result<()> {
    let pm: &PathManager = &raw_args.try_into()?;
    let (_, source_pm, target_pm) = pm.langs();
    let opts = &pm.opts;

    pm.setup_dirs()?;

    let capacity = 256 * (1 << 10); // default is 8 * (1 << 10) := 8KB
    let mut line = Vec::with_capacity(1 << 10);
    let mut irs = D::I::default();

    for pair in iter_datasets(pm) {
        let (edition, path_jsonl) = pair?;

        let reader_file = File::open(&path_jsonl)?;
        let mut reader = BufReader::with_capacity(capacity, reader_file);

        let mut line_count = 0;
        let mut accepted_count = 0;

        loop {
            line.clear();
            if reader.read_until(b'\n', &mut line)? == 0 {
                break; // EOF
            }

            line_count += 1;

            if !opts.quiet && line_count % CONSOLE_PRINT_INTERVAL == 0 {
                print!("Processed {line_count} lines...\r");
                std::io::stdout().flush()?;
            }

            // This slows down tests, since we pay the deserialization even though we
            // do not filter any entry.
            // TODO: at some point we should have a "make_dict" for CLI/release.rs
            // with a db, and another, without probing, for tests, instead of having
            // one for release.rs and other for CLI/tests.
            if dict.supports_probe() && LangCodeProbe::should_skip(&line, source_pm)? {
                continue;
            }

            let mut entry: WordEntry =
                serde_json::from_slice(&line).with_context(|| "Error decoding JSON @ make_dict")?;

            if rejected(&entry, opts) {
                continue;
            }

            accepted_count += 1;
            if accepted_count == opts.first {
                break;
            }

            if dict.skip_if(&entry) {
                continue;
            }

            let langs = Langs {
                edition,
                source: source_pm,
                target: target_pm,
            };

            dict.preprocess(langs, &mut entry, opts, &mut irs);
            dict.process(langs, &entry, &mut irs);
        }

        if !opts.quiet {
            println!("Processed {line_count} lines. Accepted {accepted_count} lines.");
        }
    }

    if !opts.quiet {
        dict.found_ir_message(pm.langs, &irs);
    }

    if irs.is_empty() {
        return Ok(());
    }

    dict.postprocess(pm.langs, &mut irs);

    opts.format.write(&dict, pm.langs, opts, pm, &irs)?;

    Ok(())
}
