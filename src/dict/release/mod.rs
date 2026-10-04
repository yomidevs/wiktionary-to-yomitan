//! Build a dictionary release.
//!
//! Publishing is done in python via release.py.
//!
//! Command to limit memory usage (linux):
//! systemd-run --user --scope -p MemoryMax=24G -p MemoryHigh=24G cargo run -r -- release -v

use std::{path::Path, time::Instant};

use anyhow::Result;
use rayon::ThreadPoolBuilder;
use rayon::prelude::*;

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
        DGlossary, DGlossaryExtended, DIpa, DIpaMerged, DMain, Dictionary, Langs, WriterFormat,
        core::skip_below_min_entries,
    },
    lang::{Edition, EditionSpec, Lang},
    path::{DictionaryType, PathManager},
};

const MAX_NUM_THREADS_MAIN: usize = 4;

/// A dictionary the release wrote, with the size of each format.
pub struct Built {
    ty: DictionaryType,
    source: String,
    target: String,
    sizes: Vec<(WriterFormat, u64)>,
}

fn size_of(path: &Path) -> u64 {
    walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(Result::ok)
        .filter_map(|entry| entry.metadata().ok())
        .filter(std::fs::Metadata::is_file)
        .map(|metadata| metadata.len())
        .sum()
}

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

    let mut built = Vec::new();
    for edition in &editions {
        built.extend(release_main(&rargs, *edition, &editions));
        built.extend(release_ipa(&rargs, *edition, &editions));
        built.extend(release_glossary(&rargs, *edition, &editions));
    }

    let targets = Lang::all();
    // let targets = [Lang::Afb];
    // let targets: Vec<Lang> = editions.iter().map(|ed| (*ed).into()).collect();
    built.par_extend(targets.par_iter().filter_map(|target| {
        // release_glossary_extended(&rargs, *target, &editions);
        release_ipa_merged(&rargs, *target, &editions)
    }));

    let elapsed = start.elapsed();
    println!("Finished dictionaries in {elapsed:.2?}");

    anyhow::ensure!(!built.is_empty(), "the release built no dictionary");

    extract_indexes(&rargs)?;

    let elapsed = start_release.elapsed();
    println!("Finished release in {elapsed:.2?}");

    write_dict_metadata(&rargs.root_dir, &editions, &built, elapsed)?;

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

fn release_main(rargs: &ReleaseArgs, edition: Edition, editions: &[Edition]) -> Vec<Built> {
    // Limit only this workload (as opposed to the full logic. IPA and glossaries are completely
    // fine and will never OOM).
    let pool = ThreadPoolBuilder::new()
        // 2 seems fine with a MemoryMax of 20GB (works on my machine TM)
        // 8 is fine for testing with only English/German/French editions
        .num_threads(MAX_NUM_THREADS_MAIN)
        .build()
        .expect("Failed to build local thread pool");

    pool.install(|| {
        Lang::all()
            .par_iter()
            .filter_map(|source| {
                // Simple English only pairs with itself
                if (edition == Edition::Simple) != (*source == Lang::Simple) {
                    return None;
                }

                let args = MainArgs {
                    langs: MainLangs {
                        source: *source,
                        target: edition,
                    },
                    dict_name: DictName::default(),
                    options: rargs.options(),
                };

                make_dict_from_db(DMain, args, editions)
                    .inspect_err(|err| tracing::error!("[main-{source}-{edition}] ERROR: {err:?}"))
                    .ok()
                    .flatten()
            })
            .collect()
    })
}

fn release_ipa(rargs: &ReleaseArgs, edition: Edition, editions: &[Edition]) -> Vec<Built> {
    Lang::all()
        .par_iter()
        .filter_map(|source| {
            // Simple English only pairs with itself
            if (edition == Edition::Simple) != (*source == Lang::Simple) {
                return None;
            }

            let args = IpaArgs {
                langs: MainLangs {
                    source: *source,
                    target: edition,
                },
                dict_name: DictName::default(),
                options: rargs.options(),
            };

            make_dict_from_db(DIpa, args, editions)
                .inspect_err(|err| tracing::error!("[ipa-{source}-{edition}] ERROR: {err:?}"))
                .ok()
                .flatten()
        })
        .collect()
}

fn release_ipa_merged(rargs: &ReleaseArgs, target: Lang, editions: &[Edition]) -> Option<Built> {
    if target == Lang::Simple {
        return None;
    }

    let args = IpaMergedArgs {
        langs: IpaMergedLangs { target },
        dict_name: DictName::default(),
        options: rargs.options(),
    };

    make_dict_from_db(DIpaMerged, args, editions)
        .inspect_err(|err| tracing::error!("[ipa-merged-{target}] ERROR: {err:?}"))
        .ok()
        .flatten()
}

fn release_glossary(rargs: &ReleaseArgs, edition: Edition, editions: &[Edition]) -> Vec<Built> {
    Lang::all()
        .par_iter()
        .filter_map(|target| {
            let langs = match (edition, target) {
                (Edition::Simple, _) | (_, Lang::Simple) => return None,
                _ if Lang::from(edition) == *target => return None,
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

            make_dict_from_db(DGlossary, args, editions)
                .inspect_err(|err| tracing::error!("[glossary-{edition}-{target}] ERROR: {err:?}"))
                .ok()
                .flatten()
        })
        .collect()
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

/// The sql selecting the entries this dictionary is built from.
///
/// Bound with the source iso, plus the target iso if it takes a second parameter.
pub trait DQuery {
    const SQL: &'static str = "SELECT entry FROM wiktextract WHERE lang = ?1";
}

impl DQuery for DMain {}
impl DQuery for DIpa {}
impl DQuery for DIpaMerged {}

/// Select entries with translations in *both* source and target.
impl DQuery for DGlossaryExtended {
    const SQL: &'static str = r"
        SELECT w.entry
        FROM wiktextract w
        JOIN translations s ON s.entry_id = w.id AND s.target_lang = ?1
        JOIN translations t ON t.entry_id = w.id AND t.target_lang = ?2
        ";
}

/// Select entries that match the source lang and have translations in target.
impl DQuery for DGlossary {
    const SQL: &'static str = r"
        SELECT w.entry
        FROM wiktextract w
        JOIN translations t ON w.id = t.entry_id
        WHERE w.lang = ?1 AND t.target_lang = ?2
        ";
}

/// Make a dictionary from database made from a Kaikki jsonlines.
pub fn make_dict_from_db<D: Dictionary + DQuery>(
    dict: D,
    raw_args: D::A,
    editions: &[Edition],
) -> Result<Option<Built>> {
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

        let mut stmt = db.conn.prepare(D::SQL)?;
        let mut rows = match stmt.parameter_count() {
            1 => stmt.query([source_pm.iso()])?,
            _ => stmt.query([source_pm.iso(), target_pm.iso()])?,
        };

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
        return Ok(None);
    }

    let mut sizes = Vec::new();
    for format in &opts.formats {
        let path = format.write(&dict, pm.langs, opts, pm, &irs)?;
        sizes.extend(path.map(|path| (*format, size_of(&path))));
    }

    let (source, target) = pm.dir_pair();
    Ok((!sizes.is_empty()).then(|| Built {
        ty: pm.dict_ty,
        source,
        target,
        sizes,
    }))
}
