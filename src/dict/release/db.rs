use std::{
    collections::HashSet,
    fs::File,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    time::Instant,
};

use anyhow::Result;
use rkyv::Archived;
use rusqlite::{Connection, params};

use crate::{lang::Edition, models::kaikki::WordEntry};

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
