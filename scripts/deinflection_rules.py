"""Scan a local copy of the yomitan repo to extract deinflection rules.

NOTE: This is an experimental sanity script at the moment, like tags.py. It prints/writes
valuable information for debugging, but does nothing that affects the dictionaries directly.

More precisely, to extract deinflection rule identifiers, stored in the "conditions" variable.
https://github.com/yomidevs/yomitan/blob/master/docs/development/language-features.md

These are found in:
ext/js/language/LANG_FOLDER/FILES.js

in the form:
const conditions = {
    'v': {
        name: 'Verb',
        isDictionaryForm: false,
        subConditions: ['v1', 'v5', 'vk', 'vs', 'vz'],
    }
}

we want to extract the conditions variables as a python dict.

Reminder:
https://github.com/yomidevs/yomitan/blob/e03bae777aa161783ce00128cdc81de221fda56f/ext/data/schemas/dictionary-term-bank-v3-schema.json
{
    "type": "string",
    "description": "String of space-separated rule identifiers for the definition which is used to validate deinflection. An empty string should be used for words which aren't inflected."
},

TODO: explain why this is important.
"""

import argparse
import json
import re
from collections import Counter, defaultdict
from pathlib import Path

type Conditions = dict[str, dict[str, str]]
"""Key is the rule ident. Values are {"name": "rule name, ...}."""


def extract_conditions(js_text: str) -> Conditions | None:
    """Extract the first `const conditions = { ... }` block from a JS file."""
    mch = re.search(r"const conditions\s*=\s*(\{.*?\});", js_text, re.DOTALL)
    if not mch:
        return None

    obj = mch.group(1)

    # Normalize JS object to JSON:
    # 1. Replace single-quoted strings with double-quoted
    obj = re.sub(r"'([^']*)'", r'"\1"', obj)
    # 2. Remove trailing commas before } or ]
    obj = re.sub(r",\s*([}\]])", r"\1", obj)
    # 3. Quote unquoted keys (e.g. isDictionaryForm: -> "isDictionaryForm":)
    obj = re.sub(r"(\b\w+\b)\s*:", r'"\1":', obj)

    json_out = json.loads(obj)
    # Remove unwanted keys: i18n and subConditions
    return {
        ident: {
            k: v
            for k, v in val.items()
            if k not in ("i18n", "subConditions", "isDictionaryForm")
        }
        for ident, val in json_out.items()
    }


def scan_yomitan_repo(repo_path: Path) -> dict[str, Conditions]:
    """Scan all language JS files and return conditions keyed by language folder."""
    if not repo_path.exists():
        raise FileNotFoundError(f"Repo not found @ {repo_path.resolve()}")

    lang_root = repo_path / "ext" / "js" / "language"
    if not lang_root.exists():
        raise FileNotFoundError(f"Language folder not found @ {lang_root}")

    results: dict[str, Conditions] = {}
    name_counts = defaultdict(Counter)
    all_conditions = defaultdict(list)

    for js_file in lang_root.rglob("*.js"):
        text = js_file.read_text(encoding="utf-8")
        conditions = extract_conditions(text)
        if not conditions:
            continue
        lang = js_file.parent.name
        for ident, condition in conditions.items():
            name = condition["name"]
            name_counts[ident][name] += 1
            all_conditions[ident].append((lang, condition))
        rel_path = js_file.relative_to(repo_path).as_posix()
        lang_result = {
            ident: {"name": cond["name"], "path": rel_path}
            for ident, cond in conditions.items()
        }
        if lang in results:
            results[lang].update(lang_result)
        else:
            results[lang] = lang_result

    # pass 1: canonical choice
    canonical = {key: counts.most_common(1)[0] for key, counts in name_counts.items()}

    # pass 2: find offenders and report
    offenders = defaultdict(list)
    for ident, entries in all_conditions.items():
        winner, _ = canonical.get(ident, (None, None))
        for lang, condition in entries:
            if condition["name"] != winner:
                offenders[ident].append((lang, condition))
    for ident, bads in offenders.items():
        cname, ccount = canonical[ident]
        print(f"OFFENDER key={ident}, canonical={cname} ({ccount})")
        for lang, cond in bads:
            print(f"  [{lang}] {cond}")
    if not offenders:
        print("Found no offenders. Rules are consistent across languages.")

    return results


def build_valid_rules_rs(res: dict[str, Conditions], out_path: Path) -> None:
    idt = " " * 4

    all_conditions: dict[str, str] = {}
    ident_langs: dict[str, list[str]] = defaultdict(list)
    ident_paths: dict[str, list[str]] = defaultdict(list)
    for lang, conditions in res.items():
        for ident, meta in conditions.items():
            all_conditions[ident] = meta["name"]
            ident_langs[ident].append(lang)
            ident_paths[ident].append(meta["path"])

    with out_path.open("w", encoding="utf-8") as f:
        w = f.write
        w("//! This file was generated and should not be edited directly.\n")
        w("//! The source code can be found at scripts/deinflection_rules.py\n\n")
        # w("//! # Rule identifiers\n")
        # w("//! | Rule | Languages | Name | Source |\n")
        # w("//! |------|-----------|------|--------|\n")
        # REPO_URL = "https://github.com/yomidevs/yomitan"
        # for ident, name in sorted(all_conditions.items()):
        #     langs = ", ".join(sorted(ident_langs[ident]))
        #     sources = " ".join(
        #         f"[{path.split('/')[-1]}]({REPO_URL}/blob/master/{path})"
        #         for path in sorted(set(ident_paths[ident]))
        #     )
        #     w(f"//! | `{ident}` | {langs} | {name} | {sources} |\n")
        w("use crate::lang::Lang;\n\n")
        w("pub fn is_valid_rule(lang: Lang, rule: &str) -> bool {\n")
        w(f"{idt}match lang {{\n")
        for lang, conditions in res.items():
            if not conditions:
                continue
            items = sorted(conditions.items())
            variants = " | ".join(f'"{ident}"' for ident, _ in items)
            w(f"{idt * 2}Lang::{lang.title()} => matches!(rule, {variants}),\n")
        w(f"{idt * 2}_ => false,\n")
        w(f"{idt}}}\n")
        w("}\n")

    print(f"Wrote rust code @ {out_path}")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "repo_path",
        type=Path,
        nargs="?",
        default="../yomitan/",  # Assumes this script is run @ wty root
        help="Path to local yomitan repo",
    )
    parser.add_argument(
        "--out", type=Path, default=None, help="Optional JSON output path"
    )
    args = parser.parse_args()

    results = scan_yomitan_repo(args.repo_path)
    res: dict[str, Conditions] = {}
    for lang, conditions in results.items():
        res[lang] = {rule: cond for rule, cond in sorted(conditions.items())}

    # TODO: snapshot some json (only once, and add it to the repo so one
    # doesn't require to have a yomitan repo copy locally)
    # build rules.rs from that snapshoted json

    if args.out:
        with args.out.open("w", encoding="utf-8") as f:
            json.dump(res, f, indent=4, ensure_ascii=False)
        print(f"Wrote results to {args.out}")

    valid_rules_rs = Path("src/dict/rules/valid.rs")
    build_valid_rules_rs(res, valid_rules_rs)


if __name__ == "__main__":
    main()
