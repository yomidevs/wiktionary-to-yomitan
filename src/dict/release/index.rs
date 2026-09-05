use std::fs::File;

use anyhow::Result;

use crate::cli::ReleaseArgs;

/// Extract indexes from dictionary zips to support dictionary updates.
///
/// Paths look like:
///   `{dict_dir}/nb/ru/wty-nb-ru.zip`
///
/// Extracted to:
///   `{index_dir}/wty-nb-ru-index.json`
///
/// The `nb/ru` folders are dropped since indexes are intended to be used
/// as direct URLs for the Yomitan upgrade machinery.
pub fn extract_indexes(rargs: &ReleaseArgs) -> Result<()> {
    let dict_dir = rargs.root_dir.join("dict");
    let index_dir = rargs.root_dir.join("index");

    std::fs::create_dir_all(&index_dir)?;
    println!("[index] Extracting indexes...");

    let mut n_indexes = 0;

    for entry in walkdir::WalkDir::new(&dict_dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|x| x == "zip"))
    {
        let zip_path = entry.path();
        let stem = zip_path.file_stem().unwrap().to_string_lossy();
        let index_path = index_dir.join(format!("{stem}-index.json"));

        let file = File::open(zip_path)?;
        let mut zip = zip::ZipArchive::new(file)?;

        anyhow::ensure!(
            zip.file_names().filter(|n| *n == "index.json").count() == 1,
            "There should be exactly one index per dictionary @ {}",
            zip_path.display()
        );

        let mut src = zip.by_name("index.json")?;
        let mut dst = File::create(&index_path)?;
        std::io::copy(&mut src, &mut dst)?;
        n_indexes += 1;
    }

    println!("[index] Extracted {n_indexes} indexes");
    Ok(())
}
