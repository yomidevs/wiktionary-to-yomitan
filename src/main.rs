use anyhow::Result;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::format::FmtSpan;

use wty::{
    cli::{Cli, Command, LangSpecs},
    db,
    dict::{
        DGlossary, DGlossaryExtended, DIpa, DIpaMerged, DMain, make_dict_from_jsonl, release, scan,
    },
    download::find_or_download_jsonl,
    lang::{Edition, Lang},
    path::PathManager,
};

fn init_logger(verbose: bool) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        if verbose {
            // Only we are set to debug. ureq and other libs stay the same.
            EnvFilter::new(format!("{}=debug", env!("CARGO_PKG_NAME")))
        } else {
            EnvFilter::new("warn")
        }
    });

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_span_events(FmtSpan::CLOSE)
        .with_timer(tracing_subscriber::fmt::time::ChronoLocal::new(
            "%H:%M:%S".to_string(),
        ))
        .init();
}

#[tracing::instrument(skip_all, level = "debug")]
fn run(cmd: Command) -> Result<()> {
    tracing::trace!("{:#?}", cmd);

    match cmd {
        Command::Main(args) => make_dict_from_jsonl(DMain, args),
        Command::Glossary(args) => make_dict_from_jsonl(DGlossary, args),
        Command::GlossaryExtended(args) => make_dict_from_jsonl(DGlossaryExtended, args),
        Command::Ipa(args) => make_dict_from_jsonl(DIpa, args),
        Command::IpaMerged(args) => make_dict_from_jsonl(DIpaMerged, args),
        Command::Download(args) => {
            // NOTE: uses MainArgs, so it expects two language codes.
            let langs: LangSpecs = args.langs.clone().try_into()?;
            let edition: Edition = langs.edition.try_into().unwrap();
            let pm = PathManager::try_from(args)?;

            let _ = std::fs::create_dir_all(pm.dir_kaik());

            let _ = find_or_download_jsonl(edition, Some(langs.source), &pm)?;
            Ok(())
        }
        Command::Iso(args) => {
            if args.edition {
                println!("{}", Lang::help_editions());
            } else {
                println!("{}", Lang::help_isos_coloured());
            }
            Ok(())
        }
        Command::Db(args) => db::run(args),
        Command::Release(args) => release(args),
        Command::Scan(args) => scan(args),
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse_cli();
    init_logger(cli.verbose);
    run(cli.command)
}
