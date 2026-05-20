use std::{fs, path::Path, sync::OnceLock};

use anyhow::{Ok, Result};

use wty::{
    cli::{DictName, GlossaryArgs, GlossaryLangs, IpaArgs, MainArgs, MainLangs, Options},
    dict::{DGlossary, DIpa, DMain, WriterFormat, make_dict_from_jsonl},
    lang::{Edition, Lang},
    path::PathManager,
};

const FIXTURE_DIR: &str = "tests";

static CASES: OnceLock<(Vec<(Lang, Lang)>, Vec<Lang>)> = OnceLock::new();

// iterdir and search for source-target-extract.jsonl files
fn cases() -> &'static (Vec<(Lang, Lang)>, Vec<Lang>) {
    CASES.get_or_init(|| {
        let fixture_input_dir = Path::new(FIXTURE_DIR).join("kaikki");
        let mut cases = Vec::new();
        let mut langs = Vec::new();

        for entry in fs::read_dir(&fixture_input_dir).unwrap().flatten() {
            let path = entry.path();
            if let Some(fname) = path.file_name().and_then(|f| f.to_str())
                && let Some(base) = fname.strip_suffix("-extract.jsonl")
                && let Some((source, target)) = base.split_once('-')
            {
                let src = source.parse::<Lang>().unwrap();
                let tar = target.parse::<Lang>().unwrap();
                cases.push((src, tar));
                if !langs.contains(&src) {
                    langs.push(src);
                }
                if !langs.contains(&tar) {
                    langs.push(tar);
                }
            }
        }
        (cases, langs)
    })
}

/// Scan folders under "root" recursively and assert no zip files are present.
fn assert_no_zip_files(root: &Path) {
    let entries = fs::read_dir(root).unwrap();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            assert_no_zip_files(&path);
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"))
        {
            panic!("zip found in tests: {}", path.display());
        }
    }
}

#[test]
fn no_zip_files_in_fixtures() {
    assert_no_zip_files(Path::new(FIXTURE_DIR));
}

fn fixture_options(fixture_dir: &Path, format: WriterFormat) -> Options {
    Options {
        pretty: true,
        experimental: false,
        root_dir: fixture_dir.to_path_buf(),
        format,
        ..Default::default()
    }
}

fn fixture_main_args(
    source: Lang,
    target: Edition,
    fixture_dir: &Path,
    format: WriterFormat,
) -> MainArgs {
    MainArgs {
        langs: MainLangs { source, target },
        dict_name: DictName::default(),
        options: fixture_options(fixture_dir, format),
    }
}

fn fixture_ipa_args(
    source: Lang,
    target: Edition,
    fixture_dir: &Path,
    format: WriterFormat,
) -> IpaArgs {
    IpaArgs {
        langs: MainLangs { source, target },
        dict_name: DictName::default(),
        options: fixture_options(fixture_dir, format),
    }
}

fn fixture_glossary_args(
    source: Edition,
    target: Lang,
    fixture_dir: &Path,
    format: WriterFormat,
) -> GlossaryArgs {
    GlossaryArgs {
        langs: GlossaryLangs { source, target },
        dict_name: DictName::default(),
        options: fixture_options(fixture_dir, format),
    }
}

/// Delete generated artifacts from previous tests runs, if any
fn delete_previous_output(pm: &PathManager) -> Result<()> {
    let pathdir_dict_temp = pm.dir_temp_dict();
    if pathdir_dict_temp.exists() {
        tracing::debug!("Deleting previous output: {pathdir_dict_temp:?}");
        fs::remove_dir_all(pathdir_dict_temp)?;
    }
    Ok(())
}

/// Run git --diff for changes in the generated json
fn check_git_diff(pm: &PathManager) -> Result<()> {
    let output = std::process::Command::new("git")
        .args([
            "diff",
            "--color=always",
            "--unified=0", // show 0 context lines
            "--",
            // we don't care about changes in tidy files
            &pm.dir_temp_dict().to_string_lossy(),
        ])
        .output()?;
    if !output.stdout.is_empty() {
        eprintln!("{}", String::from_utf8_lossy(&output.stdout));
        anyhow::bail!("changes!")
    }
    Ok(())
}

#[test]
fn snapshot_main() {
    let fixture_dir = Path::new(FIXTURE_DIR);
    let (cases, _) = cases();

    for (source, target) in cases {
        let Result::Ok(target) = (*target).try_into() else {
            continue;
        };
        let args = fixture_main_args(*source, target, fixture_dir, WriterFormat::TestYomitanMain);
        if let Err(e) = shapshot_main_go(args) {
            panic!("({source}): {e}");
        }
    }
}

/// Read the expected result in the snapshot first, then git diff
fn shapshot_main_go(margs: MainArgs) -> Result<()> {
    let pm = &PathManager::try_from(margs.clone())?;
    delete_previous_output(pm)?;
    make_dict_from_jsonl(DMain, margs)?;
    check_git_diff(pm)?;
    Ok(())
}

#[test]
fn snapshot_glossary() {
    let fixture_dir = Path::new(FIXTURE_DIR);
    let (cases, langs) = cases();

    for (source, target) in cases {
        if source != target {
            continue;
        }
        let Result::Ok(source) = (*source).try_into() else {
            continue; // skip if source is not edition
        };
        for possible_target in langs {
            if Lang::from(source) == *possible_target {
                continue;
            }
            if source == Edition::Simple || *possible_target == Lang::Simple {
                continue;
            }
            let args = fixture_glossary_args(
                source,
                *possible_target,
                fixture_dir,
                WriterFormat::TestYomitan,
            );
            make_dict_from_jsonl(DGlossary, args).unwrap();
        }
    }
}

#[test]
fn snapshot_ipa() {
    let fixture_dir = Path::new(FIXTURE_DIR);
    let (cases, _) = cases();

    for (source, target) in cases {
        let Result::Ok(target) = (*target).try_into() else {
            continue; // skip if target is not edition
        };
        let args = fixture_ipa_args(*source, target, fixture_dir, WriterFormat::TestYomitan);
        make_dict_from_jsonl(DIpa, args).unwrap();
    }
}

#[test]
fn snapshot_html_format() -> Result<()> {
    let fixture_dir = Path::new(FIXTURE_DIR);
    let format = WriterFormat::TestHtml;

    let args = fixture_main_args(Lang::En, Edition::En, fixture_dir, format);
    make_dict_from_jsonl(DMain, args).unwrap();
    let args = fixture_main_args(Lang::Ja, Edition::Ja, fixture_dir, format);
    make_dict_from_jsonl(DMain, args).unwrap();

    let args = fixture_glossary_args(Edition::Ja, Lang::En, fixture_dir, format);
    make_dict_from_jsonl(DGlossary, args).unwrap();

    let args = fixture_ipa_args(Lang::Ja, Edition::Ja, fixture_dir, format);
    make_dict_from_jsonl(DIpa, args).unwrap();

    Ok(())
}

#[test]
fn snapshot_main_stardict() -> Result<()> {
    let fixture_dir = Path::new(FIXTURE_DIR);
    let format = WriterFormat::Stardict;

    let args = fixture_main_args(Lang::En, Edition::En, fixture_dir, format);
    make_dict_from_jsonl(DMain, args).unwrap();
    let args = fixture_main_args(Lang::Ja, Edition::Ja, fixture_dir, format);
    make_dict_from_jsonl(DMain, args).unwrap();

    // This isn't really a snapshot (for now): we only test that it doesn't explode
    fs::remove_dir_all(fixture_dir.join("dict/en/en/main/stardict")).ok();
    fs::remove_dir_all(fixture_dir.join("dict/ja/ja/main/stardict")).ok();

    Ok(())
}

#[test]
fn snapshot_main_mdict() -> Result<()> {
    let fixture_dir = Path::new(FIXTURE_DIR);
    let format = WriterFormat::Mdict;

    let args = fixture_main_args(Lang::En, Edition::En, fixture_dir, format);
    make_dict_from_jsonl(DMain, args).unwrap();
    let args = fixture_main_args(Lang::Ja, Edition::Ja, fixture_dir, format);
    make_dict_from_jsonl(DMain, args).unwrap();

    // This isn't really a snapshot (for now): we only test that it doesn't explode
    fs::remove_dir_all(fixture_dir.join("dict/en/en/main/mdict")).ok();
    fs::remove_dir_all(fixture_dir.join("dict/ja/ja/main/mdict")).ok();

    Ok(())
}
