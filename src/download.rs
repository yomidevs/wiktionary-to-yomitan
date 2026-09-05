//! Utilities for downloading Kaikki jsonlines.

use std::path::PathBuf;

use anyhow::Result;

use crate::{
    lang::{Edition, Lang},
    path::{PathKind, PathManager},
    utils::skip_because_file_exists,
};

/// Return the url of the "raw" dataset.
fn url_jsonl_gz(edition: Edition) -> String {
    let root = "https://kaikki.org";

    match edition {
        Edition::En => format!("{root}/dictionary/raw-wiktextract-data.jsonl.gz"),
        other => format!("{root}/{other}wiktionary/raw-wiktextract-data.jsonl.gz"),
    }
}

/// Try to find the jsonlines in disk, otherwise download it.
pub fn find_or_download_jsonl(
    edition: Edition,
    lang: Option<Lang>,
    pm: &PathManager,
) -> Result<PathBuf> {
    let paths_candidates = pm.dataset_paths(edition, lang);
    let kinds_to_check = [PathKind::Unfiltered, PathKind::Filtered];
    let of_kind = paths_candidates.of_kind(&kinds_to_check);

    if !pm.opts.redownload
        && let Some(existing) = of_kind.iter().find(|p| p.exists())
    {
        if !pm.opts.quiet {
            skip_because_file_exists("download", existing);
        }
        return Ok(existing.clone());
    }

    let path = &of_kind.into_iter().next_back().unwrap_or_else(|| {
        panic!(
            "No path available, \
             for edition={edition:?} and lang={lang:?} | {paths_candidates:?}"
        )
    });

    #[cfg(feature = "html")]
    crate::download::download_jsonl(edition, path, false)?;

    Ok(path.clone())
}

#[cfg(feature = "html")]
pub use html::*;

#[cfg(feature = "html")]
mod html {
    use super::url_jsonl_gz;

    use anyhow::Result;
    use flate2::read::GzDecoder;
    use std::fs::File;
    use std::io::BufWriter;
    use std::path::Path;

    use crate::{
        lang::Edition,
        utils::{CHECK_C, pretty_println_at_path},
    };

    // In the past, we supported downloading the post-processed, English-edition-only,
    // filtered datasets.
    // Those became deprecated cf. <https://github.com/tatuylonen/wiktextract/issues/1178>
    // but also caused some issues due to not being structured as their "raw" counterparts.
    //
    /// Download the "raw" jsonl (jsonlines) from kaikki and write it to `path_jsonl`.
    ///
    /// "Raw" means that it does not include extra information, not intended for general use,
    /// that they (kaikki) use for their website generation.
    ///
    /// Does not write the .gz file to disk.
    ///
    /// WARN: expects `path_jsonl` to be a valid path (with existing parents etc.)
    pub fn download_jsonl(edition: Edition, path_jsonl: &Path, quiet: bool) -> Result<()> {
        let url = url_jsonl_gz(edition);
        if !quiet {
            println!("⬇ Downloading {url}");
        }

        let response = ureq::get(url).call()?;

        if let Some(last_modified) = response.headers().get("last-modified") {
            tracing::info!("Download was last modified: {:?}", last_modified);
        }

        let reader = response.into_body().into_reader();
        // We can't use gzip's ureq feature because there is no content-encoding in headers
        // https://github.com/tatuylonen/wiktextract/issues/1482
        let mut decoder = GzDecoder::new(reader);

        let mut writer = BufWriter::new(File::create(path_jsonl)?);
        std::io::copy(&mut decoder, &mut writer)?;

        if !quiet {
            pretty_println_at_path(&format!("{CHECK_C} Downloaded"), path_jsonl);
        }

        Ok(())
    }
}
