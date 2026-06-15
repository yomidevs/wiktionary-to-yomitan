build *args:
  python3 scripts/build.py {{args}}

cov:
  cargo llvm-cov --open

update *args:
  python3 scripts/update_tests.py {{args}}

# Release
release *args:
  systemd-run --user --scope -p MemoryMax=24G -p MemoryHigh=24G cargo run -r -- release {{args}}

# Publish a release made with release.rs to hugging face
publish *args:
  python3 scripts/release.py {{args}}

# Scan the release dictionaries for size information
scan:
  python3 scripts/scan.py data/release/dict

docs-serve:
  python3 scripts/generate_docs.py
  mkdocs serve

docs-publish:
  python3 scripts/generate_docs.py
  mkdocs gh-deploy

# Add a word to the testsuite
add fr to word:
  @cargo run --release -- download {{fr}} {{to}}
  rg "\"word\": \"{{word}}\"" "data/kaikki/{{to}}-extract.jsonl" -N | \
  jq -c "select(.word == \"{{word}}\" and .lang_code == \"{{fr}}\")" \
  >> "tests/kaikki/{{fr}}-{{to}}-extract.jsonl"; \

flamegraph:
  cargo flamegraph -r -- main el el -vq --skip-yomitan

stat *args:
  perf stat -d cargo run -r -- {{args}}

# Bench and log. To bench run 'cargo bench'
bench-log:
  @rm -rf target/criterion # remove cache comparisons when logging
  @cargo bench --bench benchmark > "benches/log.txt"

clippy *args:
  cargo clippy {{args}} --all-targets --all-features -- -W clippy::nursery -W clippy::pedantic \
  -A clippy::must_use_candidate \
  -A clippy::module_name_repetitions \
  -A clippy::cast_precision_loss \
  -A clippy::unicode_not_nfc

# top 20 zip files by size
size:
  find . -type f -name "*.zip" -exec du -h {} + | sort -hr | head -20
