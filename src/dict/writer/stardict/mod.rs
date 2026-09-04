use std::{
    fs::{self},
    path::PathBuf,
};

use anyhow::Result;
use pangloss::{Glossary, GlossaryInfo, Writer, formats::stardict::StardictFormat};

use crate::{
    cli::Options, dict::writer::build_entries, lang::Lang, models::yomitan::YomitanDict,
    path::PathManager,
};

mod renderer;
use renderer::StardictRenderer;

pub fn write_stardict(
    source: Lang,
    target: Lang,
    _: &Options,
    pm: &PathManager,
    ydict: YomitanDict,
) -> Result<PathBuf> {
    let dir_in_stage = pm.dir_in_stage("stardict");
    _ = fs::create_dir_all(&dir_in_stage);

    let dict_name = format!("wty-{source}-{target}");
    let ifo_path = dir_in_stage.join(format!("{dict_name}.ifo"));
    let glossary = build_glossary(&dict_name, ydict);

    StardictFormat.write(&ifo_path, &glossary)?;

    Ok(dir_in_stage)
}

// Build a Glossary out of the html rendered by StardictRenderer
fn build_glossary(dict_name: &str, ydict: YomitanDict) -> Glossary {
    // We don't need to sort entries it since pangloss does it on write:
    // https://github.com/daxida/pangloss/blob/master/src/formats/stardict/writer.rs#L66
    let entries = build_entries::<StardictRenderer>(ydict);

    let mut info = GlossaryInfo::new();
    info.insert("name", dict_name.to_string());
    info.insert("sametypesequence", "h".to_string());

    Glossary {
        entries,
        info,
        ..Default::default()
    }
}
