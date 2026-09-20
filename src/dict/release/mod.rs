//! Build a dictionary release.
//!
//! Publishing is done in python via release.py.
//!
//! Command to limit memory usage (linux):
//! systemd-run --user --scope -p MemoryMax=24G -p MemoryHigh=24G cargo run -r -- release -v

use std::time::Instant;

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
        IpaArgs, IpaMergedArgs, IpaMergedLangs, MainArgs, MainLangs, ReleaseArgs,
    },
    db::WiktextractDb,
    dict::{
        DGlossary, DGlossaryExtended, DIpa, DIpaMerged, DMain, Dictionary, Langs,
        core::skip_below_min_entries,
    },
    lang::{Edition, EditionSpec, Lang},
    path::PathManager,
};

const MAX_NUM_THREADS_MAIN: usize = 4;

/// Build a dictionary release.
pub fn release(rargs: ReleaseArgs) -> Result<()> {
    let start_release = Instant::now();
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
    download_and_create_db(&rargs, &editions);

    let start = Instant::now();

    editions.iter().for_each(|edition| {
        release_main(&rargs, *edition, &editions);
        release_ipa(&rargs, *edition, &editions);
        release_glossary(&rargs, *edition, &editions);
    });

    let targets = Lang::all();
    // let targets = [Lang::Afb];
    // let targets: Vec<Lang> = editions.iter().map(|ed| (*ed).into()).collect();
    targets.par_iter().for_each(|target| {
        release_ipa_merged(&rargs, *target, &editions);
        // release_glossary_extended(&rargs, *target, &editions);
    });

    let elapsed = start.elapsed();
    println!("Finished dictionaries in {elapsed:.2?}");

    extract_indexes(&rargs)?;

    let elapsed = start_release.elapsed();
    println!("Finished release in {elapsed:.2?}");

    write_dict_metadata(&rargs.root_dir, &editions, elapsed)?;

    Ok(())
}

fn download_and_create_db(rargs: &ReleaseArgs, editions: &[Edition]) {
    let start = Instant::now();

    let dir_kaik = rargs.root_dir.join("kaikki"); // cf. same function @ path.rs
    let _ = std::fs::create_dir(dir_kaik);

    editions.par_iter().for_each(|edition| {
        WiktextractDb::build(&rargs.root_dir, *edition, false, false).unwrap();
    });

    println!("Finished download & db creation in {:.2?}", start.elapsed());
}

fn release_main(rargs: &ReleaseArgs, edition: Edition, editions: &[Edition]) {
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
            // Simple English only pairs with itself
            if (edition == Edition::Simple) != (*source == Lang::Simple) {
                return;
            }

            let args = MainArgs {
                langs: MainLangs {
                    source: *source,
                    target: edition,
                },
                dict_name: DictName::default(),
                options: rargs.options(),
            };

            if let Err(err) = make_dict_from_db(DMain, args, editions) {
                tracing::error!("[main-{source}-{edition}] ERROR: {err:?}");
            }
        });
    });
}

fn release_ipa(rargs: &ReleaseArgs, edition: Edition, editions: &[Edition]) {
    Lang::all().par_iter().for_each(|source| {
        // Simple English only pairs with itself
        if (edition == Edition::Simple) != (*source == Lang::Simple) {
            return;
        }

        let args = IpaArgs {
            langs: MainLangs {
                source: *source,
                target: edition,
            },
            dict_name: DictName::default(),
            options: rargs.options(),
        };

        if let Err(err) = make_dict_from_db(DIpa, args, editions) {
            tracing::error!("[ipa-{source}-{edition}] ERROR: {err:?}");
        }
    });
}

fn release_ipa_merged(rargs: &ReleaseArgs, target: Lang, editions: &[Edition]) {
    if target == Lang::Simple {
        return;
    }

    let args = IpaMergedArgs {
        langs: IpaMergedLangs { target },
        dict_name: DictName::default(),
        options: rargs.options(),
    };

    if let Err(err) = make_dict_from_db(DIpaMerged, args, editions) {
        tracing::error!("[ipa-merged-{target}] ERROR: {err:?}");
    }
}

fn release_glossary(rargs: &ReleaseArgs, edition: Edition, editions: &[Edition]) {
    Lang::all().par_iter().for_each(|target| {
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
            options: rargs.options(),
        };

        if let Err(err) = make_dict_from_db(DGlossary, args, editions) {
            tracing::error!("[glossary-{edition}-{target}] ERROR: {err:?}");
        }
    });
}

#[allow(unused)]
fn release_glossary_extended(rargs: &ReleaseArgs, source: Lang, editions: &[Edition]) {
    Lang::all().par_iter().for_each(|target| {
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
            options: rargs.options(),
        };

        if let Err(err) = make_dict_from_db(DGlossaryExtended, args, editions) {
            tracing::error!("[gloss-all-{source}-{target}] ERROR: {err:?}");
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

/// Select entries with translations in *both* source and target.
impl DQuery for DGlossaryExtended {
    fn statement_str() -> &'static str {
        r"
        SELECT w.entry
        FROM wiktextract w
        JOIN translations s ON s.entry_id = w.id AND s.target_lang = ?1
        JOIN translations t ON t.entry_id = w.id AND t.target_lang = ?2
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

    dict.postprocess(pm.langs, &mut irs);

    if skip_below_min_entries(&irs, pm) {
        return Ok(());
    }

    opts.format.write(&dict, pm.langs, opts, pm, &irs)?;

    Ok(())
}
