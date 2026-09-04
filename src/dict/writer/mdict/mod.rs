use std::{
    fs::{self},
    path::PathBuf,
};

use anyhow::Result;
use pangloss::{DataEntry, Glossary, GlossaryInfo, Writer, formats::mdict::MdictFormat};

use crate::{
    cli::Options,
    dict::writer::{STYLES_CSS, YOMITAN_CSS, build_entries},
    lang::Lang,
    models::yomitan::YomitanDict,
    path::PathManager,
};

mod renderer;
use renderer::MdictRenderer;

pub fn write_mdict(
    source: Lang,
    target: Lang,
    _: &Options,
    pm: &PathManager,
    ydict: YomitanDict,
) -> Result<PathBuf> {
    let dir_in_stage = pm.dir_in_stage("mdict");
    _ = fs::create_dir_all(&dir_in_stage);

    let dict_name = format!("wty-{source}-{target}");
    let mdx_path = dir_in_stage.join(format!("{dict_name}.mdx"));
    let glossary = build_glossary(&dict_name, ydict);

    MdictFormat::default().write(&mdx_path, &glossary)?;

    Ok(dir_in_stage)
}

fn build_glossary(dict_name: &str, ydict: YomitanDict) -> Glossary {
    let entries = build_entries::<MdictRenderer>(ydict);

    // In theory we could call this in pangloss
    let data_entries = vec![
        DataEntry::new("styles.css", STYLES_CSS.to_vec()),
        DataEntry::new("yomitan.css", YOMITAN_CSS.to_vec()),
    ];

    let mut info = GlossaryInfo::new();
    info.insert("name", dict_name.to_string());

    Glossary {
        entries,
        data_entries,
        info,
        ..Default::default()
    }
}
