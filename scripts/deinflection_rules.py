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


def extract_conditions(js_text: str) -> dict | None:
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
        key: {
            k: v
            for k, v in val.items()
            if k not in ("i18n", "subConditions", "isDictionaryForm")
        }
        for key, val in json_out.items()
    }


def scan_yomitan_repo(repo_path: Path) -> dict[str, dict]:
    """Scan all language JS files and return conditions keyed by language folder."""
    if not repo_path.exists():
        raise FileNotFoundError(f"Repo not found @ {repo_path.resolve()}")

    lang_root = repo_path / "ext" / "js" / "language"
    if not lang_root.exists():
        raise FileNotFoundError(f"Language folder not found @ {lang_root}")

    results: dict[str, dict] = {}
    name_counts = defaultdict(Counter)
    all_conditions = defaultdict(list)

    for js_file in lang_root.rglob("*.js"):
        text = js_file.read_text(encoding="utf-8")
        conditions = extract_conditions(text)
        if not conditions:
            continue
        lang = js_file.parent.name
        for key, condition in conditions.items():
            name = condition["name"]
            name_counts[key][name] += 1
            all_conditions[key].append((lang, condition))
            # Merge by language (if/when there are multiple transform files for it)
            if lang in results:
                results[lang].update(conditions)
            else:
                results[lang] = conditions

    # pass 1: canonical choice
    canonical = {key: counts.most_common(1)[0] for key, counts in name_counts.items()}

    # pass 2: find offenders and report
    offenders = defaultdict(list)
    for key, entries in all_conditions.items():
        winner, _ = canonical.get(key, (None, None))
        for lang, condition in entries:
            if condition["name"] != winner:
                offenders[key].append((lang, condition))
    for key, bads in offenders.items():
        cname, ccount = canonical[key]
        print(f"OFFENDER key={key}, canonical={cname} ({ccount})")
        for lang, cond in bads:
            print(f"  [{lang}] {cond}")
    if not offenders:
        print("Found no offenders. Rules are consistent across languages.")

    return results


def build_rules_rs(res: dict[str, list[str]], out_path: Path) -> None:
    """Generate src/dict/rules.rs from the extracted conditions."""
    idt = " " * 4

    with out_path.open("w", encoding="utf-8") as f:
        w = f.write
        w("//! This file was generated and should not be edited directly.\n")
        w("//! The source code can be found at scripts/scan_yomitan.py\n\n")
        w("use crate::lang::Lang;\n\n")
        w("pub fn is_valid_rule(lang: Lang, rule: &str) -> bool {\n")
        w(f"{idt}match lang {{\n")

        for lang, conditions in res.items():
            if not conditions:
                continue
            variants = " | ".join(f'"{c}"' for c in conditions)
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

    if args.out:
        # with args.out.open("w", encoding="utf-8") as f:
        #     json.dump(results, f, indent=4, ensure_ascii=False)
        res = {}
        for k, v in results.items():
            if k not in res:
                res[k] = []
            for k1 in v.keys():
                res[k].append(k1)
        print(res)
        with args.out.open("w", encoding="utf-8") as f:
            json.dump(res, f, indent=4, ensure_ascii=False)

        print(f"Wrote results to {args.out}")

        rules_rs = Path("src/dict/rules.rs")
        build_rules_rs(res, rules_rs)


if __name__ == "__main__":
    main()
