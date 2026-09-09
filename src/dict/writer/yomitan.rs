//! Shared write behaviour.
//!
//! Structs that are dictionary dependent, like the intermediate representation or diagnostics, are
//! not included here and should be next to their dictionary for visibility.

use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::{Ok, Result};
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use crate::{
    cli::Options,
    dict::{
        index::get_index,
        writer::{STYLES_CSS, STYLES_CSS_EXPERIMENTAL},
    },
    lang::Lang,
    models::yomitan::{YomitanDict, YomitanEntry},
    path::PathManager,
    tags::get_tag_bank_as_tag_info,
    utils::pretty_print_at_path,
};

const BANK_SIZE: usize = 25_000;

enum Sink<'a> {
    Disk,
    Zip(&'a mut ZipWriter<File>, SimpleFileOptions),
}

// no metadata - writes to disk
pub fn write_test_yomitan(opts: &Options, pm: &PathManager, ydict: YomitanDict) -> Result<PathBuf> {
    let out_dir = pm.dir_temp_dict();
    fs::create_dir_all(&out_dir)?;

    let mut bank_index = 0;
    for group in ydict.into_iter_grouped() {
        write_banks(
            opts.pretty,
            opts.quiet,
            &group.entries,
            &mut bank_index,
            group.label,
            &out_dir,
            Sink::Disk,
        )?;
    }

    Ok(out_dir)
}

/// Write a [`YomitanDict`] to a zip sink, along with metadata (index, css etc.).
pub fn write_yomitan(
    source: Lang,
    target: Lang,
    opts: &Options,
    pm: &PathManager,
    ydict: YomitanDict,
) -> Result<PathBuf> {
    let writer_path = pm.path_dict();
    let writer_file = File::create(&writer_path)?;
    let mut zip = ZipWriter::new(writer_file);
    let zip_opts =
        SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    // Zip index.json
    let index_string = get_index(pm.dict_ty, &pm.dict_name_expanded(), source, target);
    zip.start_file("index.json", zip_opts)?;
    zip.write_all(index_string.as_bytes())?;

    // Zip a copy of styles.css
    zip.start_file("styles.css", zip_opts)?;
    if opts.experimental {
        zip.write_all(STYLES_CSS_EXPERIMENTAL)?;
    } else {
        zip.write_all(STYLES_CSS)?;
    }

    // Zip a (potentially localized) version without aliases of tag_bank_term.json
    let tag_bank = get_tag_bank_as_tag_info(target);
    let tag_bank_bytes = serde_json::to_vec_pretty(&tag_bank)?;
    zip.start_file("tag_bank_1.json", zip_opts)?; // it needs to end in _1
    zip.write_all(&tag_bank_bytes)?;

    let mut bank_index = 0;
    for group in ydict.into_iter_grouped() {
        write_banks(
            opts.pretty,
            opts.quiet,
            &group.entries,
            &mut bank_index,
            group.label,
            &writer_path,
            Sink::Zip(&mut zip, zip_opts),
        )?;
    }

    zip.finish()?;

    Ok(writer_path)
}

/// Writes `yomitan_entries` in banks to a sink (either disk or zip).
#[tracing::instrument(skip_all, level = "DEBUG")]
fn write_banks(
    pretty: bool,
    quiet: bool,
    yomitan_entries: &[YomitanEntry],
    bank_index: &mut usize,
    label: &'static str,
    out_dir: &Path,
    mut sink: Sink,
) -> Result<()> {
    // NOTE: this assumes that once a type is passed, all the remaining entries are of same type
    let bank_name_prefix = match yomitan_entries.first() {
        Some(first) => first.file_prefix(),
        None => return Ok(()),
    };

    let banks: Vec<_> = yomitan_entries.chunks(BANK_SIZE).collect();
    let total_bank_num = banks.len();

    let first_index = *bank_index;
    *bank_index += total_bank_num;
    let bank_name =
        |bank_num: usize| format!("{bank_name_prefix}_{}.json", first_index + bank_num + 1);

    let report = |bank_num: usize, bank: &[YomitanEntry], name: &str| -> Result<()> {
        if quiet {
            return Ok(());
        }
        if bank_num > 0 {
            print!("\r\x1b[K");
        }
        pretty_print_at_path(
            &format!(
                "Wrote yomitan {label} bank {}/{total_bank_num} ({} entries)",
                bank_num + 1,
                bank.len()
            ),
            out_dir.join(name),
        );
        std::io::stdout().flush()?;
        Ok(())
    };

    match sink {
        Sink::Disk => {
            for (bank_num, bank) in banks.iter().enumerate() {
                let name = bank_name(bank_num);
                let json_bytes = to_json(pretty, bank)?;
                File::create(out_dir.join(&name))?.write_all(&json_bytes)?;
                report(bank_num, bank, &name)?;
            }
        }
        Sink::Zip(ref mut zip, zip_options) => {
            for (bank_num, bank) in banks.iter().enumerate() {
                let name = bank_name(bank_num);
                let json_bytes = to_json(pretty, bank)?;
                zip.start_file(&name, zip_options)?;
                zip.write_all(&json_bytes)?;
                report(bank_num, bank, &name)?;
            }
        }
    }

    if !quiet {
        println!();
    }

    Ok(())
}

fn to_json(pretty: bool, bank: &[YomitanEntry]) -> Result<Vec<u8>> {
    let json_bytes = if pretty {
        serde_json::to_vec_pretty(&bank)?
    } else {
        serde_json::to_vec(&bank)?
    };
    Ok(json_bytes)
}
