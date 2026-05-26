//! Command line interface.

use std::{fmt, ops::Deref, path::PathBuf, str::FromStr};

use anyhow::{Ok, Result, bail};
use clap::{Parser, Subcommand};

use crate::{
    dict::{Langs, WriterFormat},
    lang::{Edition, EditionSpec, Lang},
    models::kaikki::WordEntry,
    path::{DictionaryType, PathManager},
};

#[derive(Debug, Parser)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    // NOTE: the order in which this --verbose flag appears in subcommands help seems cursed.
    //
    /// Verbose output (set logging level to DEBUG)
    #[arg(long, short, global = true)]
    pub verbose: bool,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Main dictionary. Uses target for the edition
    Main(MainArgs),

    /// Short dictionary made from translations. Uses source for the edition
    Glossary(GlossaryArgs),

    /// Short dictionary made from translations. Supports any language pair
    GlossaryExtended(GlossaryExtendedArgs),

    /// Phonetic transcription dictionary. Uses target for the edition
    Ipa(IpaArgs),

    /// Phonetic transcription dictionary. Uses all editions
    IpaMerged(IpaMergedArgs),

    /// Download a Kaikki jsonlines
    Download(MainArgs),

    /// Show supported iso codes, with coloured editions
    Iso(IsoArgs),

    /// Build a release with all dictionaries
    Release(ReleaseArgs),

    /// Diagnostic utility
    Scan(MainLangs),
}

#[derive(Parser, Debug, Clone)]
pub struct MainArgs {
    #[command(flatten)]
    pub langs: MainLangs,

    /// Dictionary name
    #[arg(default_value_t)]
    pub dict_name: DictName,

    #[command(flatten)]
    pub options: Options,
}

#[derive(Parser, Debug)]
pub struct GlossaryArgs {
    #[command(flatten)]
    pub langs: GlossaryLangs,

    /// Dictionary name
    #[arg(default_value_t)]
    pub dict_name: DictName,

    #[command(flatten)]
    pub options: Options,
}

#[derive(Parser, Debug)]
pub struct GlossaryExtendedArgs {
    #[command(flatten)]
    pub langs: GlossaryExtendedLangs,

    /// Dictionary name
    #[arg(default_value_t)]
    pub dict_name: DictName,

    #[command(flatten)]
    pub options: Options,
}

#[derive(Parser, Debug)]
pub struct IpaArgs {
    #[command(flatten)]
    pub langs: MainLangs,

    /// Dictionary name
    #[arg(default_value_t)]
    pub dict_name: DictName,

    #[command(flatten)]
    pub options: Options,
}

#[derive(Parser, Debug)]
pub struct IpaMergedArgs {
    #[command(flatten)]
    pub langs: IpaMergedLangs,

    /// Dictionary name
    #[arg(default_value_t)]
    pub dict_name: DictName,

    #[command(flatten)]
    pub options: Options,
}

#[derive(Parser, Debug)]
pub struct ReleaseArgs {
    /// Change the root directory
    #[arg(long, default_value = "data")]
    pub root_dir: PathBuf,
}

#[derive(Parser, Debug, Default)]
pub struct IsoArgs {
    /// Only print languages with edition
    #[arg(long)]
    pub edition: bool,
}

/// Langs-like struct that validates edition for `target` and skips `edition`.
#[derive(Parser, Debug, Clone)]
pub struct MainLangs {
    /// Source language
    pub source: Lang,

    /// Target language (edition)
    pub target: Edition,
}

/// Langs-like struct that validates edition for `source` and skips `edition`.
#[derive(Parser, Debug, Clone)]
pub struct GlossaryLangs {
    /// Source language (edition)
    pub source: Edition,

    /// Target language
    pub target: Lang,
}

/// Langs-like struct that validates edition for `edition`.
#[derive(Parser, Debug, Clone)]
pub struct GlossaryExtendedLangs {
    /// Edition language
    pub edition: EditionSpec,

    /// Source language
    pub source: Lang,

    /// Target language
    pub target: Lang,
}

/// Langs-like struct that only takes one language.
#[derive(Parser, Debug, Clone)]
pub struct IpaMergedLangs {
    /// Target language
    pub target: Lang,
}

#[expect(clippy::struct_excessive_bools)]
#[derive(Parser, Debug, Default, Clone)]
pub struct Options {
    /// Redownload kaikki files
    #[arg(long, short)]
    pub redownload: bool,

    /// Only keep the first n filtered lines. -1 keeps all
    #[arg(long, default_value_t = -1)]
    pub first: i32,

    // Example:
    //   `--filter pos,adv`
    //
    // You can specify this option multiple times:
    //   `--filter pos,adv --filter word,foo`
    //
    /// Only keep entries matching certain key–value filters
    #[arg(long, value_parser = parse_tuple)]
    pub filter: Vec<(FilterKey, String)>,

    // Example:
    //   `--reject pos,adj`
    //
    // You can specify this option multiple times:
    //   `--reject pos,adj --reject word,foo`
    //
    /// Only keep entries not matching certain key–value filters
    #[arg(long, value_parser = parse_tuple)]
    pub reject: Vec<(FilterKey, String)>,

    /// Do not print anything to the console
    #[arg(long, short)]
    pub quiet: bool,

    /// Write jsons with whitespace
    #[arg(short, long)]
    pub pretty: bool,

    /// Include experimental features
    #[arg(short, long)]
    pub experimental: bool,

    /// Change the root directory
    #[arg(long, default_value = "data")]
    pub root_dir: PathBuf,

    /// Writer format
    #[arg(long, default_value_t = WriterFormat::Yomitan)]
    pub format: WriterFormat,
}

/// Newtype string wrapper to overwrite Default with `wty`.
#[derive(Debug, Clone)]
pub struct DictName(String);

impl Default for DictName {
    fn default() -> Self {
        Self(String::from("wty"))
    }
}

impl FromStr for DictName {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Result::Ok(Self(String::from(s)))
    }
}

impl fmt::Display for DictName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Deref for DictName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

fn parse_tuple(s: &str) -> Result<(FilterKey, String), String> {
    let parts: Vec<_> = s.split(',').map(|x| x.trim().to_string()).collect();
    if parts.len() != 2 {
        return Err("expected two comma-separated values".into());
    }
    let filter_key = FilterKey::try_from(parts[0].as_str()).map_err(|e| e.to_string())?;
    core::result::Result::Ok((filter_key, parts[1].clone()))
}

/// A key used to filter a [`WordEntry`] by one of its fields.
#[derive(Debug, Clone)]
pub enum FilterKey {
    LangCode,
    Word,
    Pos,
}

impl FilterKey {
    pub fn field_value<'a>(&self, entry: &'a WordEntry) -> &'a str {
        match self {
            Self::LangCode => &entry.lang_code,
            Self::Word => &entry.word,
            Self::Pos => &entry.pos,
        }
    }

    fn try_from(s: &str) -> Result<Self> {
        match s {
            "lang_code" => Ok(Self::LangCode),
            "word" => Ok(Self::Word),
            "pos" => Ok(Self::Pos),
            other => bail!("unknown filter key '{other}'. Choose between: lang_code | word | pos"),
        }
    }
}

fn check_simple_english(source: &Lang, target: &Lang) -> Result<()> {
    match (source, target) {
        (Lang::Simple, Lang::Simple) => Ok(()),
        (Lang::Simple, _) | (_, Lang::Simple) => {
            anyhow::bail!("Simple English must be used as both source and target.")
        }
        _ => Ok(()),
    }
}

// Only the main dictionary makes sense with Simple English
fn err_on_simple_english(source: &Lang, target: &Lang) -> Result<()> {
    match (source, target) {
        (Lang::Simple, _) | (_, Lang::Simple) => {
            anyhow::bail!("Simple English can not be used for this dictionary.")
        }
        _ => Ok(()),
    }
}

fn err_on_source_being_target(source: &Lang, target: &Lang) -> Result<()> {
    if source == target {
        anyhow::bail!("in a glossary dictionary source must be different from target.");
    }
    Ok(())
}

impl Cli {
    pub fn parse_cli() -> Self {
        Self::parse()
    }
}

/// Unified language configuration. See [`crate::dict::Langs`].
#[derive(Debug, Clone, Copy)]
pub struct LangSpecs {
    pub edition: EditionSpec,
    pub source: Lang,
    pub target: Lang,
}

impl From<Langs> for LangSpecs {
    fn from(value: Langs) -> Self {
        Self {
            edition: EditionSpec::One(value.edition),
            source: value.source,
            target: value.target,
        }
    }
}

impl TryFrom<MainLangs> for LangSpecs {
    type Error = anyhow::Error;

    fn try_from(langs: MainLangs) -> Result<Self> {
        check_simple_english(&langs.source, &langs.target.into())?;

        Ok(Self {
            edition: EditionSpec::One(langs.target),
            source: langs.source,
            target: langs.target.into(),
        })
    }
}

impl TryFrom<GlossaryLangs> for LangSpecs {
    type Error = anyhow::Error;

    fn try_from(langs: GlossaryLangs) -> Result<Self> {
        err_on_simple_english(&langs.source.into(), &langs.target)?;
        err_on_source_being_target(&langs.source.into(), &langs.target)?;

        Ok(Self {
            edition: EditionSpec::One(langs.source),
            source: langs.source.into(),
            target: langs.target,
        })
    }
}

impl TryFrom<GlossaryExtendedLangs> for LangSpecs {
    type Error = anyhow::Error;

    fn try_from(langs: GlossaryExtendedLangs) -> Result<Self> {
        err_on_simple_english(&langs.source, &langs.target)?;
        err_on_source_being_target(&langs.source, &langs.target)?;

        Ok(Self {
            edition: langs.edition,
            source: langs.source,
            target: langs.target,
        })
    }
}

impl TryFrom<IpaMergedLangs> for LangSpecs {
    type Error = anyhow::Error;

    fn try_from(langs: IpaMergedLangs) -> Result<Self> {
        err_on_simple_english(&langs.target, &langs.target)?;

        Ok(Self {
            edition: EditionSpec::All,
            source: langs.target, // Not used
            target: langs.target,
        })
    }
}

macro_rules! impl_try_into_pathmanager {
    ($ty:ty, $dict_ty:expr) => {
        impl TryFrom<$ty> for PathManager {
            type Error = anyhow::Error;

            fn try_from(args: $ty) -> Result<Self> {
                Ok(Self {
                    dict_ty: $dict_ty,
                    dict_name: args.dict_name,
                    langs: args.langs.try_into()?,
                    opts: args.options,
                })
            }
        }
    };
}

impl_try_into_pathmanager!(MainArgs, DictionaryType::Main);
impl_try_into_pathmanager!(GlossaryArgs, DictionaryType::Glossary);
impl_try_into_pathmanager!(GlossaryExtendedArgs, DictionaryType::GlossaryExtended);
impl_try_into_pathmanager!(IpaArgs, DictionaryType::Ipa);
impl_try_into_pathmanager!(IpaMergedArgs, DictionaryType::IpaMerged);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_commands() {
        assert!(Cli::try_parse_from(["wty", "main", "el", "en"]).is_ok());
        assert!(Cli::try_parse_from(["wty", "glossary", "el", "en"]).is_ok());
    }

    #[test]
    fn main_needs_target_edition() {
        assert!(Cli::try_parse_from(["wty", "main", "grc", "el"]).is_ok());
        assert!(Cli::try_parse_from(["wty", "main", "el", "grc"]).is_err());
    }

    #[test]
    fn glossary_needs_source_edition() {
        assert!(Cli::try_parse_from(["wty", "glossary", "grc", "el"]).is_err());
        assert!(Cli::try_parse_from(["wty", "glossary", "el", "grc"]).is_ok());
    }

    #[test]
    fn glossary_can_not_be_monolingual() {
        let res = Cli::try_parse_from(["wty", "glossary", "el", "el"]);
        let cli = res.unwrap(); // The parsing should be ok
        if let Command::Glossary(glossary_args) = cli.command {
            assert!(LangSpecs::try_from(glossary_args.langs).is_err());
        } else {
            panic!()
        }

        let res = Cli::try_parse_from(["wty", "glossary-extended", "all", "el", "el"]);
        let cli = res.unwrap(); // The parsing should be ok
        if let Command::GlossaryExtended(glossary_args) = cli.command {
            assert!(LangSpecs::try_from(glossary_args.langs).is_err());
        } else {
            panic!()
        }
    }

    #[test]
    fn filter_flag() {
        assert!(MainArgs::try_parse_from(["_pname", "el", "el", "--filter", "foo,bar"]).is_err());
        assert!(MainArgs::try_parse_from(["_pname", "el", "el", "--filter", "word,hello"]).is_ok());
        assert!(MainArgs::try_parse_from(["_pname", "el", "el", "--reject", "pos,name"]).is_ok());
    }
}
