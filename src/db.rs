use std::{
    collections::HashSet,
    fs::File,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use rayon::prelude::*;
use rkyv::Archived;
use rusqlite::{Connection, params};

use crate::{
    cli::{DbArgs, DbOp, DictName, MainArgs, MainLangs, Options},
    download::find_or_download_jsonl,
    lang::Edition,
    models::kaikki::WordEntry,
    path::PathManager,
};

pub struct WiktextractDb {
    pub conn: Connection,
}

impl WiktextractDb {
    /// Path to the folder that contains the databases for all editions.
    fn db_folder<P>(root_dir: P) -> PathBuf
    where
        P: AsRef<Path>,
    {
        root_dir.as_ref().join("db")
    }

    /// Path for the database of this edition.
    fn db_path_for<P>(root_dir: P, edition: Edition) -> PathBuf
    where
        P: AsRef<Path>,
    {
        Self::db_folder(root_dir).join(format!("wiktextract_{edition}.db"))
    }

    pub fn open<P>(root_dir: P, edition: Edition) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let db_path = Self::db_path_for(root_dir, edition);
        let conn = Connection::open(&db_path)?;
        Ok(Self { conn })
    }

    /// Delete the database of `edition`. Returns whether there was one to delete.
    fn remove<P>(root_dir: P, edition: Edition) -> Result<bool>
    where
        P: AsRef<Path>,
    {
        let db_path = Self::db_path_for(root_dir, edition);
        let existed = db_path.exists();

        for suffix in ["", "-journal", "-wal", "-shm"] {
            let path = db_path.with_file_name(format!(
                "{}{suffix}",
                db_path.file_name().unwrap_or_default().to_string_lossy()
            ));
            match std::fs::remove_file(&path) {
                Ok(()) => tracing::debug!("Removed {}", path.display()),
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(err) => {
                    return Err(err).with_context(|| format!("removing {}", path.display()));
                }
            }
        }

        Ok(existed)
    }

    /// Resolve the jsonl of `edition` (downloading it if missing) and import it.
    ///
    /// Returns how long the import took, which is what the release metadata records.
    pub fn build<P>(root_dir: P, edition: Edition, force: bool, quiet: bool) -> Result<Duration>
    where
        P: AsRef<Path>,
    {
        let root_dir = root_dir.as_ref();

        // Only used to resolve where the jsonl of this edition lives.
        let args = MainArgs {
            langs: MainLangs {
                source: edition.into(),
                target: edition,
            },
            dict_name: DictName::default(),
            options: Options {
                quiet,
                root_dir: root_dir.to_path_buf(),
                ..Default::default()
            },
        };
        let pm: PathManager = args.try_into()?;

        let now = Instant::now();
        let path_jsonl = find_or_download_jsonl(edition, None, &pm)?;
        if !quiet {
            println!("Finished download for {edition} ({:.2?})", now.elapsed());
        }

        if force {
            Self::remove(root_dir, edition)?;
        }

        let now = Instant::now();
        Self::create(root_dir, edition, path_jsonl)?;
        let elapsed = now.elapsed();
        if !quiet {
            println!("Finished database for {edition} ({elapsed:.2?})");
        }

        Ok(elapsed)
    }

    pub fn create<P>(root_dir: P, edition: Edition, path_jsonl: PathBuf) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let _ = std::fs::create_dir(Self::db_folder(&root_dir));

        let db_path = Self::db_path_for(&root_dir, edition);
        let conn = Connection::open(&db_path)?;

        conn.execute_batch(
            r"
            CREATE TABLE IF NOT EXISTS wiktextract (
                id INTEGER PRIMARY KEY,
                lang TEXT NOT NULL,
                entry BLOB NOT NULL
            );

            CREATE TABLE IF NOT EXISTS translations (
                entry_id INTEGER NOT NULL,
                target_lang TEXT NOT NULL,
                FOREIGN KEY(entry_id) REFERENCES wiktextract(id)
            );

            CREATE INDEX IF NOT EXISTS idx_wiktextract_lang
            ON wiktextract(lang);

            CREATE INDEX IF NOT EXISTS idx_translations_target_lang
            ON translations(target_lang);

            CREATE INDEX IF NOT EXISTS idx_translations_entry_id
            ON translations(entry_id);
            ",
        )?;

        let mut db = Self { conn };

        // NOTE: Not sure if we need to check that we init the db beforehand
        let count: i64 = db
            .conn
            .query_row("SELECT COUNT(*) FROM wiktextract", [], |row| row.get(0))?;

        if count == 0 {
            tracing::info!("DB empty for {edition}, importing JSONL...");
            db.import_jsonl(path_jsonl)?;
        } else {
            tracing::trace!("DB already initialized for {edition} ({count} rows)");
        }

        Ok(db)
    }

    #[tracing::instrument(skip_all, level = "debug")]
    pub fn import_jsonl<P: AsRef<Path>>(&mut self, jsonl_path: P) -> Result<()> {
        let start = Instant::now();
        let file = File::open(&jsonl_path)?;
        let reader = BufReader::new(file);

        let tx = self.conn.transaction()?;
        {
            let mut insert_entry =
                tx.prepare("INSERT INTO wiktextract (lang, entry) VALUES (?, ?)")?;

            let mut insert_translation =
                tx.prepare("INSERT INTO translations (entry_id, target_lang) VALUES (?, ?)")?;

            for line in reader.lines() {
                let line = line?;
                let word_entry: WordEntry = serde_json::from_str(&line)?;
                let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&word_entry)?;

                // We are fine with adding entries for unsupported languages because we support
                // almost everything, and the remaining percentage is very low.
                // It takes more time to filter the unsupported languages than to ignore them.

                insert_entry.execute(params![word_entry.lang_code, bytes.as_ref()])?;

                let entry_id = tx.last_insert_rowid();

                let mut seen = HashSet::new();
                for trans in word_entry.translations {
                    if seen.insert(trans.lang_code.clone()) {
                        insert_translation.execute(params![entry_id, trans.lang_code])?;
                    }
                }
            }
        }
        tx.commit()?;

        tracing::debug!(
            "Making db took {:.3} ms",
            start.elapsed().as_secs_f64() * 1000.0
        );

        Ok(())
    }

    pub fn blob_to_word_entry(blob: &[u8]) -> Result<WordEntry> {
        let archived: &Archived<WordEntry> =
            rkyv::access::<Archived<WordEntry>, rkyv::rancor::Error>(blob)?;
        let word_entry: WordEntry = rkyv::deserialize::<WordEntry, rkyv::rancor::Error>(archived)?;
        Ok(word_entry)
    }
}

/// Run a `wty db` subcommand.
pub fn run(args: DbArgs) -> Result<()> {
    match args.op {
        DbOp::Build(args) => {
            let editions = args.select.editions();
            let root_dir = &args.select.root_dir;
            let _ = std::fs::create_dir_all(root_dir.join("kaikki"));

            let start = Instant::now();
            editions.par_iter().try_for_each(|edition| {
                WiktextractDb::build(root_dir, *edition, args.force, false).map(|_| ())
            })?;
            println!(
                "Built {} database(s) in {:.2?}",
                editions.len(),
                start.elapsed()
            );
        }
        DbOp::Drop(args) => {
            for edition in args.editions() {
                let existed = WiktextractDb::remove(&args.root_dir, edition)?;
                if existed {
                    println!("[{edition}] dropped");
                } else {
                    println!("[{edition}] not built");
                }
            }
        }
    }

    Ok(())
}
