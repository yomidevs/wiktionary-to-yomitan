use std::fmt;

use anyhow::Result;
use clap::ValueEnum;

use crate::{
    cli::{LangSpecs, Options},
    dict::{Dictionary, Intermediate},
    path::PathManager,
    utils::{CHECK_C, pretty_println_at_path},
};

mod debug_forms;
mod html;
mod mdict;
mod renderer;
mod stardict;
mod yomitan;

use debug_forms::write_debug_forms;
use html::{write_html, write_test_html};
use mdict::write_mdict;
use stardict::write_stardict;
use yomitan::{write_test_yomitan, write_yomitan};

pub const STYLES_CSS: &[u8] = include_bytes!("../../../assets/styles/styles.css");
pub const STYLES_CSS_EXPERIMENTAL: &[u8] =
    include_bytes!("../../../assets/styles/styles_experimental.css");
pub const YOMITAN_CSS: &[u8] = include_bytes!("../../../assets/styles/styles_yomitan.css");

#[derive(ValueEnum, Debug, Default, Clone, Copy)]
pub enum WriterFormat {
    // Yomitan zipped
    #[default]
    Yomitan,
    // Write irs as json
    Ir,
    // Simple html that matches the Yomitan structure
    Html,
    // Mdict format
    Mdict,
    // Stardict format
    Stardict,
    // Debug inflections (only useful for the main dict)
    DebugForms,

    // Simple html, no css
    #[value(skip)]
    TestHtml,
    // Yomitan unzipped (json) + no metadata (index.json etc.)
    #[value(skip)]
    TestYomitan,
    // Self::YomitanSimple + Self::Ir
    #[value(skip)]
    TestYomitanMain,
    // Skip writing (for benchmarking etc.)
    Skip,
}

impl fmt::Display for WriterFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Yomitan => "yomitan",
            Self::Ir => "ir",
            Self::Html => "html",
            Self::Mdict => "mdict",
            Self::Stardict => "stardict",
            Self::DebugForms => "debug-forms",
            Self::TestHtml => "test-html",
            Self::TestYomitan => "test-yomitan",
            Self::TestYomitanMain => "test-yomitan-main",
            Self::Skip => "skip",
        })
    }
}

impl WriterFormat {
    pub fn write<D: Dictionary>(
        self,
        dict: &D,
        langs: LangSpecs,
        opts: &Options,
        pm: &PathManager,
        irs: &D::I,
    ) -> Result<()> {
        let wrote_at_path = match self {
            Self::Yomitan => write_yomitan(
                langs.source,
                langs.target,
                opts,
                pm,
                dict.to_yomitan(langs, irs),
            )?,
            Self::Ir => irs.write(pm)?,
            Self::Html => write_html(opts, pm, dict.to_yomitan(langs, irs))?,
            Self::Mdict => write_mdict(
                langs.source,
                langs.target,
                opts,
                pm,
                dict.to_yomitan(langs, irs),
            )?,
            Self::Stardict => write_stardict(
                langs.source,
                langs.target,
                opts,
                pm,
                dict.to_yomitan(langs, irs),
            )?,
            Self::DebugForms => write_debug_forms(opts, pm, dict.to_yomitan(langs, irs))?,

            // We don't need to pretty print a message for these.
            Self::TestHtml => {
                write_test_html(opts, pm, dict.to_yomitan(langs, irs))?;
                return Ok(());
            }
            Self::TestYomitan => {
                write_test_yomitan(opts, pm, dict.to_yomitan(langs, irs))?;
                return Ok(());
            }
            Self::TestYomitanMain => {
                irs.write(pm)?;
                write_test_yomitan(opts, pm, dict.to_yomitan(langs, irs))?;
                return Ok(());
            }
            Self::Skip => return Ok(()),
        };

        if !opts.quiet {
            pretty_println_at_path(&format!("{CHECK_C} Wrote dict"), wrote_at_path);
        }

        Ok(())
    }
}
