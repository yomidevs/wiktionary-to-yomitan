//! Main dictionary.

mod ir;
mod locale;
mod yomitan;

pub use ir::get_reading;

use crate::{
    cli::{LangSpecs, MainArgs, Options},
    dict::{Dictionary, Langs},
    models::{kaikki::WordEntry, yomitan::YomitanDict},
};

#[derive(Debug, Clone, Copy)]
pub struct DMain;

impl Dictionary for DMain {
    type I = ir::Tidy;
    type A = MainArgs;

    fn skip_if(&self, entry: &WordEntry) -> bool {
        ir::should_skip_entry(entry)
    }

    fn preprocess(&self, langs: Langs, entry: &mut WordEntry, opts: &Options, irs: &mut Self::I) {
        ir::preprocess_main(langs.edition, langs.source, opts, entry, irs);
    }

    fn process(&self, langs: Langs, entry: &WordEntry, irs: &mut Self::I) {
        ir::process_main(langs.edition, langs.source, entry, irs);
    }

    fn postprocess(&self, langs: LangSpecs, irs: &mut Self::I) {
        ir::postprocess_main(langs, irs);
    }

    fn found_ir_message(&self, langs: LangSpecs, irs: &Self::I) {
        ir::found_ir_message_impl(langs, irs);
    }

    fn to_yomitan(&self, langs: LangSpecs, irs: &Self::I) -> YomitanDict {
        yomitan::to_yomitan_impl(langs, irs)
    }
}
