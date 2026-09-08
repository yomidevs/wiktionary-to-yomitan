//! Build a dictionary release.
//!
//! Publishing is done in python via release.py.
//!
//! Command to limit memory usage (linux):
//! systemd-run --user --scope -p MemoryMax=24G -p MemoryHigh=24G cargo run -r -- release -v

use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};

use anyhow::Result;
use rayon::ThreadPoolBuilder;
use rayon::prelude::*;
use rusqlite::{Rows, Statement};

mod index;
mod metadata;

use index::extract_indexes;
use metadata::write_dict_metadata;

use crate::{
    cli::{
        DictName, GlossaryArgs, GlossaryExtendedArgs, GlossaryExtendedLangs, GlossaryLangs,
        IpaArgs, IpaMergedArgs, IpaMergedLangs, MainArgs, MainLangs, Options, ReleaseArgs,
    },
    db::WiktextractDb,
    dict::{
        DGlossary, DGlossaryExtended, DIpa, DIpaMerged, DMain, Dictionary, Intermediate, Langs,
    },
    lang::{Edition, EditionSpec, Lang},
    path::PathManager,
};

const MAX_NUM_THREADS_MAIN: usize = 4;

#[derive(Debug, Default)]
struct TimingStats {
    timings: Mutex<HashMap<String, Duration>>,
}

impl TimingStats {
    fn new() -> Self {
        Self {
            timings: Mutex::new(HashMap::new()),
        }
    }

    fn record(&self, key: String, duration: Duration) {
        self.timings.lock().unwrap().insert(key, duration);
    }
}

/// Build a dictionary release.
pub fn release(rargs: ReleaseArgs) -> Result<()> {
    let editions = rargs.editions();

    println!("rargs: {rargs:?}");
    println!("Making release with {} editions", editions.len());
    println!(
        "- {}",
        editions
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    );

    // First, download all jsonlines to prevent races when creating databases.
    //
    // NOTE: For some reason this takes time even when db are init, why?
    let _ = std::fs::create_dir(&rargs.root_dir);
    let db_stats = TimingStats::new();
    download_and_create_db(&rargs, &editions, &db_stats);

    let start = Instant::now();
    let stats = TimingStats::new();

    editions.iter().for_each(|edition| {
        release_main(&rargs, *edition, &editions, &stats);
        release_ipa(&rargs, *edition, &editions, &stats);
        release_glossary(&rargs, *edition, &editions, &stats);
    });

    let targets = Lang::all();
    // let targets = [Lang::Afb];
    // let targets: Vec<Lang> = editions.iter().map(|ed| (*ed).into()).collect();
    targets.par_iter().for_each(|target| {
        release_ipa_merged(&rargs, *target, &editions, &stats);
        // release_glossary_extended(*target, &editions, &stats);
    });

    let elapsed = start.elapsed();
    println!("Finished dictionaries in {elapsed:.2?}");

    extract_indexes(&rargs)?;

    if rargs.editions.is_empty() {
        write_dict_metadata(&rargs.root_dir, &db_stats, &stats)?;
    } else {
        println!("[meta] Skipped metadata: this release was scoped to specific editions");
    }

    Ok(())
}

fn download_and_create_db(rargs: &ReleaseArgs, editions: &[Edition], stats: &TimingStats) {
    let start = Instant::now();

    let dir_kaik = rargs.root_dir.join("kaikki"); // cf. same function @ path.rs
    let _ = std::fs::create_dir(dir_kaik);

    editions.par_iter().for_each(|edition| {
        let elapsed = WiktextractDb::build(&rargs.root_dir, *edition, false, false).unwrap();
        stats.record(edition.to_string(), elapsed);
    });

    println!("Finished download & db creation in {:.2?}", start.elapsed());
}

// Pretty print utility
fn pp(
    dict_name: &str,
    first_lang: Lang,
    second_lang: Option<Lang>,
    time: Instant,
    stats: &TimingStats,
) {
    let duration = time.elapsed();

    let key = match second_lang {
        Some(second_lang) => format!("{dict_name}-{first_lang}-{second_lang}"),
        None => format!("{dict_name}-{first_lang}"),
    };

    stats.record(key.clone(), duration);

    // let label = format!("[{}]", key);
    // eprintln!("{label:<20} done in {:.2?}", time.elapsed());
}

fn release_main(rargs: &ReleaseArgs, edition: Edition, editions: &[Edition], stats: &TimingStats) {
    // Limit only this workload (as opposed to the full logic. IPA and glossaries are completely
    // fine and will never OOM).
    let pool = ThreadPoolBuilder::new()
        // 2 seems fine with a MemoryMax of 20GB (works on my machine TM)
        // 8 is fine for testing with only English/German/French editions
        .num_threads(MAX_NUM_THREADS_MAIN)
        .build()
        .expect("Failed to build local thread pool");

    pool.install(|| {
        Lang::all().par_iter().for_each(|source| {
            let start = Instant::now();

            let langs = match (edition, source) {
                (Edition::Simple, Lang::Simple) => MainLangs {
                    source: *source,
                    target: edition,
                },
                (Edition::Simple, _) | (_, Lang::Simple) => return,
                _ => MainLangs {
                    source: *source,
                    target: edition,
                },
            };

            let args = MainArgs {
                langs,
                dict_name: DictName::default(),
                options: Options {
                    quiet: true,
                    root_dir: rargs.root_dir.clone(),
                    format: rargs.format,
                    ..Default::default()
                },
            };

            match make_dict_from_db(DMain, args, editions) {
                Ok(()) => pp("main", *source, Some(edition.into()), start, stats),
                Err(err) => tracing::error!("[main-{source}-{edition}] ERROR: {err:?}"),
            }
        });
    });
}

fn release_ipa(rargs: &ReleaseArgs, edition: Edition, editions: &[Edition], stats: &TimingStats) {
    Lang::all().par_iter().for_each(|source| {
        let start = Instant::now();

        let langs = match (edition, source) {
            (Edition::Simple, Lang::Simple) => MainLangs {
                source: *source,
                target: edition,
            },
            (Edition::Simple, _) | (_, Lang::Simple) => return,
            _ => MainLangs {
                source: *source,
                target: edition,
            },
        };

        let args = IpaArgs {
            langs,
            dict_name: DictName::default(),
            options: Options {
                quiet: true,
                root_dir: rargs.root_dir.clone(),
                format: rargs.format,
                ..Default::default()
            },
        };

        match make_dict_from_db(DIpa, args, editions) {
            Ok(()) => pp("ipa", *source, Some(edition.into()), start, stats),
            Err(err) => tracing::error!("[ipa-{source}-{edition}] ERROR: {err:?}"),
        }
    });
}

fn release_ipa_merged(
    rargs: &ReleaseArgs,
    target: Lang,
    editions: &[Edition],
    stats: &TimingStats,
) {
    let start = Instant::now();

    let langs = match target {
        Lang::Simple => return,
        _ => IpaMergedLangs { target },
    };

    let args = IpaMergedArgs {
        langs,
        dict_name: DictName::default(),
        options: Options {
            quiet: true,
            root_dir: rargs.root_dir.clone(),
            format: rargs.format,
            ..Default::default()
        },
    };

    match make_dict_from_db(DIpaMerged, args, editions) {
        Ok(()) => pp("ipa-merged", target, None, start, stats),
        Err(err) => tracing::error!("[ipa-merged-{target}] ERROR: {err:?}"),
    }
}

fn release_glossary(
    rargs: &ReleaseArgs,
    edition: Edition,
    editions: &[Edition],
    stats: &TimingStats,
) {
    Lang::all().par_iter().for_each(|target| {
        let start = Instant::now();

        let langs = match (edition, target) {
            (Edition::Simple, _) | (_, Lang::Simple) => return,
            _ if Lang::from(edition) == *target => return,
            _ => GlossaryLangs {
                source: edition,
                target: *target,
            },
        };

        let args = GlossaryArgs {
            langs,
            dict_name: DictName::default(),
            options: Options {
                quiet: true,
                root_dir: rargs.root_dir.clone(),
                format: rargs.format,
                ..Default::default()
            },
        };

        match make_dict_from_db(DGlossary, args, editions) {
            // Reverse order of main/ipa
            Ok(()) => pp("glossary", edition.into(), Some(*target), start, stats),
            Err(err) => tracing::error!("[glossary-{edition}-{target}] ERROR: {err:?}"),
        }
    });
}

#[allow(unused)]
fn release_glossary_extended(source: Lang, editions: &[Edition], stats: &TimingStats) {
    Lang::all().par_iter().for_each(|target| {
        let start = Instant::now();

        let langs = match (source, target) {
            (Lang::Simple, _) | (_, Lang::Simple) => return,
            _ if source == *target => return,
            _ => GlossaryExtendedLangs {
                edition: EditionSpec::All,
                source,
                target: *target,
            },
        };

        let args = GlossaryExtendedArgs {
            langs,
            dict_name: DictName::default(),
            options: Options {
                quiet: true,
                root_dir: "data".into(),
                ..Default::default()
            },
        };

        match make_dict_from_db(DGlossaryExtended, args, editions) {
            Ok(()) => pp("gloss-all", source, Some(*target), start, stats),
            Err(err) => tracing::error!("[gloss-all-{source}-{target}] ERROR: {err:?}"),
        }
    });
}

/// Implementation of the sql query.
///
/// Defaults to selecting entries that match the source lang.
pub trait DQuery {
    fn statement_str() -> &'static str {
        "SELECT entry FROM wiktextract WHERE lang = ?1"
    }

    fn query<'a>(
        stmt: &'a mut Statement,
        source: &str,
        _target: &str,
    ) -> rusqlite::Result<Rows<'a>> {
        stmt.query([source])
    }
}

impl DQuery for DMain {}
impl DQuery for DIpa {}
impl DQuery for DIpaMerged {}
impl DQuery for DGlossaryExtended {}

/// Select entries that match the source lang and have translations in target.
impl DQuery for DGlossary {
    fn statement_str() -> &'static str {
        r"
        SELECT w.entry
        FROM wiktextract w
        JOIN translations t ON w.id = t.entry_id
        WHERE w.lang = ?1 AND t.target_lang = ?2
        "
    }

    fn query<'a>(
        stmt: &'a mut Statement,
        source: &str,
        target: &str,
    ) -> rusqlite::Result<rusqlite::Rows<'a>> {
        stmt.query([source, target])
    }
}

/// Make a dictionary from database made from a Kaikki jsonlines.
pub fn make_dict_from_db<D: Dictionary + DQuery>(
    dict: D,
    raw_args: D::A,
    editions: &[Edition],
) -> Result<()> {
    let pm: &PathManager = &raw_args.try_into()?;
    let (edition_pm, source_pm, target_pm) = pm.langs();
    let opts = &pm.opts;
    pm.setup_dirs()?;

    tracing::trace!("{pm:#?}");

    let mut irs = D::I::default();

    for edition in edition_pm
        .variants()
        .into_iter()
        .filter(|e| editions.contains(e))
    {
        let db = WiktextractDb::open(&opts.root_dir, edition)?;
        let langs = Langs {
            edition,
            source: source_pm,
            target: target_pm,
        };

        let mut stmt = db.conn.prepare(D::statement_str())?;
        let mut rows = D::query(&mut stmt, source_pm.iso(), target_pm.iso())?;

        while let Some(row) = rows.next()? {
            let blob: &[u8] = row.get_ref(0)?.as_blob()?;
            let mut entry = WiktextractDb::blob_to_word_entry(blob)?;

            if dict.skip_if(&entry) {
                continue;
            }

            dict.preprocess(langs, &mut entry, opts, &mut irs);
            dict.process(langs, &entry, &mut irs);
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
