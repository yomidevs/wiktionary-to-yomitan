//! A binary and library crate to make Yomitan dictionaries from Kaikki jsonlines.

pub mod cli;
pub mod db;
pub mod dict;
pub mod download;
pub mod lang;
pub mod models;
pub mod path;
mod tags;
mod utils;

use fxhash::FxBuildHasher;
use indexmap::{IndexMap, IndexSet};

type Map<K, V> = IndexMap<K, V, FxBuildHasher>;
type Set<K> = IndexSet<K, FxBuildHasher>;
