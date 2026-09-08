//! Dictionary processing pipeline.

mod core;
mod glossary;
mod index;
mod ipa;
mod main;
mod release;
mod rules;
mod scan;
mod writer;

use core::LangCodeProbe;
pub use core::{Dictionary, Intermediate, Langs, make_dict_from_jsonl};

// Dictionary types
pub use glossary::{DGlossary, DGlossaryExtended};
pub use ipa::{DIpa, DIpaMerged};
pub use main::DMain;

pub use release::{make_dict_from_db, release};
pub use scan::scan;
pub use writer::WriterFormat;
