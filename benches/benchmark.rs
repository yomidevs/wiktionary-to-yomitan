use std::path::Path;

use criterion::{Criterion, criterion_group, criterion_main};

use wty::{
    cli::{DictName, MainArgs, MainLangs, Options},
    dict::{DMain, WriterFormat, make_dict_from_jsonl},
    lang::{Edition, Lang},
    path::PathManager,
};

const BENCH_FIXTURES_DIR_100: &str = "benches/fixtures";

fn fixture_options(fixture_dir: &Path) -> Options {
    Options {
        pretty: true,
        experimental: false,
        quiet: true,
        root_dir: fixture_dir.to_path_buf(),
        format: WriterFormat::Ir,
        ..Default::default()
    }
}

fn fixture_main_args(source: Lang, target: Edition, fixture_path: &Path) -> MainArgs {
    MainArgs {
        langs: MainLangs { source, target },
        dict_name: DictName::default(),
        options: fixture_options(fixture_path),
    }
}

fn bench_monolingual(c: &mut Criterion, edition: Edition, label: &str) {
    let fixture_path = Path::new(BENCH_FIXTURES_DIR_100);
    let args = fixture_main_args(edition.into(), edition, fixture_path);
    let pm: PathManager = args.clone().try_into().unwrap();

    c.bench_function(label, |b| {
        b.iter(|| make_dict_from_jsonl(DMain, args.clone()));
    });

    std::fs::remove_dir_all(pm.dir_dicts()).unwrap();
}

fn bench_el_el(c: &mut Criterion) {
    bench_monolingual(c, Edition::El, "main_dict_el_el");
}

fn bench_de_de(c: &mut Criterion) {
    bench_monolingual(c, Edition::De, "main_dict_de_de");
}

criterion_group!(benches, bench_el_el, bench_de_de);
criterion_main!(benches);
