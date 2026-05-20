"""Generate rust source files from assets.

Requires python 3.12+
"""

import argparse
import json
import re
import sys
from dataclasses import astuple, dataclass
from pathlib import Path
from typing import Any


@dataclass
class Lang:
    iso: str
    language: str
    display_name: str
    flag: str
    # https://github.com/tatuylonen/wiktextract/tree/master/src/wiktextract/extractor
    has_edition: bool

    @property
    def ident(self) -> str:
        """Identifier in rust code. Removes forbidden chars."""
        return self.iso.replace("-", "").title()


@dataclass
class WhitelistedTag:
    short_tag: str
    category: str
    sort_order: int
    # if array, first element will be used, others are aliases
    long_tag_aliases: str | list[str]
    popularity_score: int

    @property
    def ident(self) -> str:
        """Identifier in rust code. Removes forbidden chars."""
        return (
            self.long_tag().replace("-", " ").replace("_", " ").title().replace(" ", "")
        )

    def __post_init__(self) -> None:
        check_valid_short_tag(self.short_tag)
        for tag in self.longs_as_list():
            check_valid_long_tag(tag)
        if isinstance(self.long_tag_aliases, list):
            assert len(self.long_tag_aliases) > 1, (
                f"Invalid long_tag_aliases for '{self.short_tag}': "
                "list must contain more than one element. "
                "Use a string instead if there is only one alias."
            )

    def longs_as_list(self) -> list[str]:
        if isinstance(self.long_tag_aliases, str):
            return [self.long_tag_aliases]
        return self.long_tag_aliases

    def long_tag(self) -> str:
        return self.longs_as_list()[0]


@dataclass
class TagTranslation:
    long_tag_en: str
    short_tag: str
    long_tag: str

    def __post_init__(self) -> None:
        # No need to check for long_tag_en, an invalid char means it's not in tag_bank
        check_valid_short_tag(self.short_tag)
        check_valid_long_tag(self.long_tag)


type Locale = dict[
    str,  # iso
    list[TagTranslation],
]

INVALID_SHORT_TAG_CHARS = ' ;/"\\\n\r\t'  # Forbid spaces: yomitan will split them
INVALID_LONG_TAG_CHARS = ';/"\\\n\r\t'


def check_valid_short_tag(tag: str) -> None:
    _check_valid_tag(tag, INVALID_SHORT_TAG_CHARS)


def check_valid_long_tag(tag: str) -> None:
    _check_valid_tag(tag, INVALID_LONG_TAG_CHARS)


def _check_valid_tag(tag: str, invalid_chars: str) -> None:
    invalid = [c for c in tag if c in invalid_chars]
    assert not invalid, (
        f"Invalid tag '{tag}': contains forbidden character(s): "
        f"{', '.join(repr(c) for c in set(invalid))}"
    )


def write_warning(f) -> None:
    f.write("//! This file was generated and should not be edited directly.\n")
    f.write("//! The source code can be found at scripts/build.py\n\n")


def generate_tags_rs(
    tag_order: list[str],
    whitelisted_tags: list[WhitelistedTag],
    f,
) -> None:
    # Having duplicated short tags is pointless, use aliases instead.
    seen = {}
    for wt in whitelisted_tags:
        st = wt.short_tag
        if st in seen:
            old = seen[st]
            print(f"ERROR: duplicated short tag\n{wt}\n{old}")
            sys.exit(1)
        else:
            seen[st] = wt

    # Having duplicated long tags is pointless, only the first will be found by find_map.
    seen = {}
    for wt in whitelisted_tags:
        lt = wt.long_tag()
        if lt in seen:
            old = seen[lt]
            print(f"ERROR: duplicated long tag\n{wt}\n{old}")
            sys.exit(1)
        else:
            seen[lt] = wt

    idt = " " * 4
    w = f.write  # shorthand

    write_warning(f)

    w(f"pub const TAG_ORDER: [&str; {len(tag_order)}] = [\n")
    for tag in tag_order:
        w(f'{idt}"{tag}",\n')
    w("];\n\n")

    # Not sure why all of this was done in the original, it makes almost no sense

    w("#[rustfmt::skip]\n")
    w(
        f"pub static TAG_BANK: [(&str, &str, i32, &[&str], i32); {len(whitelisted_tags)}] = [\n"
    )
    for wt in whitelisted_tags:
        longs_str = str(wt.longs_as_list()).replace("'", '"')
        w(
            f'{idt}("{wt.short_tag}", "{wt.category}", {wt.sort_order}, &{longs_str}, {wt.popularity_score}),\n'
        )
    w("];\n\n")

    poses = [wt for wt in whitelisted_tags if wt.category == "partOfSpeech"]

    # Note that there is a difference between:
    # 1. the pos that wiktextract normalizes
    # 2. the short version we decide upon
    # 3. The long version we decide upon.
    #
    # Sometimes 1. matches 3., for example for "noun" (short: n);
    # and sometimes 1. matches 2., for example, for "prep" (long: preprosition)

    # Enum definition
    w("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\n")
    w("pub enum Pos {\n")
    for pos in poses:
        w(f"{idt}{pos.ident},\n")
    # w(f"{idt}Other(Box<str>),\n")
    w(f"{idt}Unknown,\n")
    w("}\n\n")

    # From<&str>
    w("impl From<&str> for Pos {\n")
    w(f"{idt}fn from(s: &str) -> Self {{\n")
    w(f"{idt * 2}match s {{\n")
    for pos in poses:
        choices = " | ".join(f'"{long}"' for long in pos.longs_as_list())
        w(f"{idt * 3}{choices} => Self::{pos.ident},\n")
    # w(f"{idt * 3}_ => Self::Other(s.into()),\n")
    w(f"{idt * 3}_ => Self::Unknown,\n")
    w(f"{idt * 2}}}\n")
    w(f"{idt}}}\n")
    w("}\n\n")

    # long
    w("impl Pos {\n")
    w(f"{idt}pub const fn long(&self) -> &str {{\n")
    w(f"{idt * 2}match self {{\n")
    for pos in poses:
        w(f'{idt * 3}Self::{pos.ident} => "{pos.long_tag()}",\n')
    # w(f"{idt * 3}Self::Other(s) => s,\n")
    w(f'{idt * 3}Self::Unknown => "unknown",\n')
    w(f"{idt * 2}}}\n")
    w(f"{idt}}}\n")
    w("}\n\n")

    # short
    w("impl Pos {\n")
    w(f"{idt}pub const fn short(&self) -> &str {{\n")
    w(f"{idt * 2}match self {{\n")
    for pos in poses:
        w(f'{idt * 3}Self::{pos.ident} => "{pos.short_tag}",\n')
    # w(f"{idt}{idt}{idt}Self::Other(s) => s,\n")
    w(f'{idt * 3}Self::Unknown => "?",\n')
    w(f"{idt * 2}}}\n")
    w(f"{idt}}}\n")
    w("}\n\n")

    # Serde::serialize as sort
    w("impl serde::Serialize for Pos {\n")
    w(
        f"{idt}fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {{\n"
    )
    w(f"{idt * 2}serializer.serialize_str(self.long())\n")
    w(f"{idt}}}\n")
    w("}\n")


def generate_lang_rs(langs: list[Lang], f) -> None:
    idt = " " * 4
    w = f.write  # shorthand

    f.write("//! Abstractions over language codes.\n")
    f.write("//!\n")

    write_warning(f)

    # w("#![rustfmt::skip]\n")

    w("""use std::{
    fmt::{Debug, Display},
    hash::Hash,
    str::FromStr,
};\n\n""")

    w("use serde::{Deserialize, Serialize};\n\n")

    ### Trait

    shared_traits = [
        "Clone",
        "Debug",
        "Display",
        "FromStr",
        "AsRef<str>",
        "PartialEq",
        "Eq",
        "Hash",
    ]
    w("// The idea is from https://github.com/johnstonskj/rust-codes/tree/main\n")
    w("//\n")
    w("/// Helper trait to ensure that some other traits are implemented.\n")
    w(f"pub trait Code: {' + '.join(shared_traits)} {{}}\n\n")
    w("impl Code for Lang {}\n")
    w("impl Code for EditionSpec {}\n")
    w("impl Code for Edition {}\n")
    w("\n")

    ### Lang start

    # Lang
    w("#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]\n")
    w("pub enum Lang {\n")
    for lang in langs:
        w(f"{idt}/// {lang.language}\n")  # doc
        w(f"{idt}{lang.ident},\n")
    w("}\n\n")

    # Lang: From<Edition>
    w("impl From<Edition> for Lang {\n")
    w(f"{idt}fn from(value: Edition) -> Self {{\n")
    w(f"{idt * 2}match value {{\n")
    for lang in langs:
        if lang.has_edition:
            w(f"{idt * 3}Edition::{lang.ident} => Self::{lang.ident},\n")
    w(f"{idt * 2}}}\n")
    w(f"{idt}}}\n")
    w("}\n\n")

    w("impl Lang {\n")

    # Lang: help_messages
    is_supported = " | ".join(lang.iso for lang in langs)
    fn_name = "help_isos"
    w(f"{idt}pub const fn {fn_name}() -> &'static str {{\n")
    w(f'{idt * 2}"Supported isos: {is_supported}"\n')
    w(f"{idt}}}\n\n")

    coloured_parts = [
        f"\x1b[32m{lang.iso}\x1b[0m" if lang.has_edition else lang.iso for lang in langs
    ]
    isos_colored = " | ".join(coloured_parts)
    w(f"{idt}pub const fn help_isos_coloured() -> &'static str {{\n")
    w(f'{idt * 2}"Supported isos: {isos_colored}"\n')
    w(f"{idt}}}\n\n")

    with_edition = " | ".join(lang.iso for lang in langs if lang.has_edition)
    w(f"{idt}pub const fn help_editions() -> &'static str {{\n")
    w(f'{idt * 2}"Supported editions: {with_edition}"\n')
    w(f"{idt}}}\n\n")

    # Lang: long. long: Lang::El => "Greek"
    w(f"{idt}pub const fn long(&self) -> &'static str {{\n")
    w(f"{idt * 2}match self {{\n")
    for lang in langs:
        w(f'{idt * 3}Self::{lang.ident} => "{lang.language}",\n')
    w(f"{idt * 2}}}\n")
    w(f"{idt}}}\n\n")

    # Lang: all (iteration)
    w(f"{idt}pub fn all() -> Vec<Self> {{\n")
    w(f"{idt * 2}vec![\n")
    for lang in langs:
        w(f"{idt * 3}Self::{lang.ident},\n")
    w(f"{idt * 2}]\n")
    w(f"{idt}}}\n")
    w("}\n\n")

    # Lang: TryInto<Edition>
    w("impl TryInto<Edition> for Lang {\n")
    w(f"{idt}type Error = &'static str;\n\n")
    w(f"{idt}fn try_into(self) -> Result<Edition, Self::Error> {{\n")
    w(f"{idt * 2}match self {{\n")
    for lang in langs:
        if lang.has_edition:
            w(f"{idt * 3}Self::{lang.ident} => Ok(Edition::{lang.ident}),\n")
    w(f'{idt * 3}_ => Err("language has no edition"),\n')
    w(f"{idt * 2}}}\n")
    w(f"{idt}}}\n")
    w("}\n\n")

    # Lang: FromStr
    w("impl FromStr for Lang {\n")
    w(f"{idt}type Err = String;\n\n")
    w(f"{idt}fn from_str(s: &str) -> Result<Self, Self::Err> {{\n")
    w(f"{idt * 2}match s.to_lowercase().as_str() {{\n")
    for lang in langs:
        w(f'{idt * 3}"{lang.iso.lower()}" => Ok(Self::{lang.ident}),\n')
    w(
        f"{idt * 3}_ => Err(format!(\"unsupported iso code '{{s}}'\\n{{}}\", Self::{fn_name}())),\n"
    )
    w(f"{idt * 2}}}\n")
    w(f"{idt}}}\n")
    w("}\n\n")

    # Lang: AsRef<&str>
    w("impl AsRef<str> for Lang {\n")
    w(f"{idt}fn as_ref(&self) -> &str {{\n")
    w(f"{idt * 2}match self {{\n")
    for lang in langs:
        w(f'{idt * 3}Self::{lang.ident} => "{lang.iso.lower()}",\n')
    w(f"{idt * 2}}}\n")
    w(f"{idt}}}\n")
    w("}\n\n")

    # Lang: iso - to separate lang_code filtering from json path fetching logic
    w("impl Lang {\n")
    w(f"{idt}pub fn iso(&self) -> &str {{\n")
    w(f"{idt * 2}match self {{\n")
    w(f'{idt * 3}Self::Simple => "en",\n')
    w(f"{idt * 3}_ => self.as_ref(),\n")
    w(f"{idt * 2}}}\n")
    w(f"{idt}}}\n")
    w("}\n\n")

    # Lang: Display
    w("impl Display for Lang {\n")
    w(f"{idt}fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {{\n")
    w(f"{idt * 2}f.write_str(self.as_ref())\n")
    w(f"{idt}}}\n")
    w("}\n\n")

    ### EditionSpec start

    # EditionSpec
    w("#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]\n")
    w("pub enum EditionSpec {\n")
    w(f"{idt}/// All editions\n")
    w(f"{idt}All,\n")
    w(f"{idt}/// An `Edition`\n")
    w(f"{idt}One(Edition),\n")
    w("}\n\n")

    # EditionSpec: variants (iteration)
    w("impl EditionSpec {\n")
    w(f"{idt}pub fn variants(&self) -> Vec<Edition> {{\n")
    w(f"{idt * 2}match self {{\n")
    w(f"{idt * 3}Self::All => Edition::all(),\n")
    w(f"{idt * 3}Self::One(lang) => vec![*lang],\n")
    w(f"{idt * 2}}}\n")
    w(f"{idt}}}\n")
    w("}\n\n")

    # EditionSpec: From<Edition>
    w("impl From<Edition> for EditionSpec {\n")
    w(f"{idt}fn from(val: Edition) -> Self {{\n")
    w(f"{idt * 2}Self::One(val)\n")
    w(f"{idt}}}\n")
    w("}\n\n")

    # EditionSpec: TryInto<Edition>
    w("impl TryInto<Edition> for EditionSpec {\n")
    w(f"{idt}type Error = &'static str;\n\n")
    w(f"{idt}fn try_into(self) -> Result<Edition, Self::Error> {{\n")
    w(f"{idt * 2}match self {{\n")
    w(f'{idt * 3}Self::All => Err("cannot convert from All"),\n')
    w(f"{idt * 3}Self::One(lang) => Ok(lang),\n")
    w(f"{idt * 2}}}\n")
    w(f"{idt}}}\n")
    w("}\n\n")

    # EditionSpec: FromStr
    w("impl FromStr for EditionSpec {\n")
    w(f"{idt}type Err = String;\n\n")
    w(f"{idt}fn from_str(s: &str) -> Result<Self, Self::Err> {{\n")
    w(f"{idt * 2}match s {{\n")
    w(f'{idt * 3}"all" => Ok(Self::All),\n')
    w(f"{idt * 3}other => Ok(Self::One(Edition::from_str(other)?)),\n")
    w(f"{idt * 2}}}\n")
    w(f"{idt}}}\n")
    w("}\n\n")

    # EditionSpec: AsRef<&str>
    w("impl AsRef<str> for EditionSpec {\n")
    w(f"{idt}fn as_ref(&self) -> &str {{\n")
    w(f"{idt * 2}match self {{\n")
    w(f'{idt * 3}Self::All => "all",\n')
    w(f"{idt * 3}Self::One(lang) => lang.as_ref(),\n")
    w(f"{idt * 2}}}\n")
    w(f"{idt}}}\n")
    w("}\n\n")

    # EditionSpec: Display
    w("impl Display for EditionSpec {\n")
    w(f"{idt}fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {{\n")
    w(f"{idt * 2}f.write_str(self.as_ref())\n")
    w(f"{idt}}}\n")
    w("}\n\n")

    ### Edition start

    # Edition
    w("#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]\n")
    w("pub enum Edition {\n")
    for lang in langs:
        if lang.has_edition:
            w(f"{idt}/// {lang.language}\n")  # doc
            w(f"{idt}{lang.ident},\n")
    w("}\n\n")

    # Edition: all (iteration)
    w("impl Edition {\n")
    w(f"{idt}pub fn all() -> Vec<Self> {{\n")
    w(f"{idt * 2}vec![\n")
    for lang in langs:
        if lang.has_edition:
            w(f"{idt * 3}Self::{lang.ident},\n")
    w(f"{idt * 2}]\n")
    w(f"{idt}}}\n")
    w("}\n\n")

    # Edition: FromStr
    w("impl FromStr for Edition {\n")
    w(f"{idt}type Err = String;\n\n")
    w(f"{idt}fn from_str(s: &str) -> Result<Self, Self::Err> {{\n")
    w(f"{idt * 2}match s.to_lowercase().as_str() {{\n")
    for lang in langs:
        if lang.has_edition:
            w(f'{idt * 3}"{lang.iso.lower()}" => Ok(Self::{lang.ident}),\n')
    w(f"{idt * 3}_ => Err(format!(\"invalid edition '{{s}}'\")),\n")
    w(f"{idt * 2}}}\n")
    w(f"{idt}}}\n")
    w("}\n\n")

    # Edition: AsRef<&str>
    w("impl AsRef<str> for Edition {\n")
    w(f"{idt}fn as_ref(&self) -> &str {{\n")
    w(f"{idt * 2}match self {{\n")
    for lang in langs:
        if lang.has_edition:
            w(f'{idt * 3}Self::{lang.ident} => "{lang.iso.lower()}",\n')
    w(f"{idt * 2}}}\n")
    w(f"{idt}}}\n")
    w("}\n\n")

    # Edition: iso - to separate lang_code filtering from json path fetching logic
    w("impl Edition {\n")
    w(f"{idt}pub fn iso(&self) -> &str {{\n")
    w(f"{idt * 2}match self {{\n")
    w(f'{idt * 3}Self::Simple => "en",\n')
    w(f"{idt * 3}_ => self.as_ref(),\n")
    w(f"{idt * 2}}}\n")
    w(f"{idt}}}\n")
    w("}\n\n")

    # Edition: Display
    w("impl Display for Edition {\n")
    w(f"{idt}fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {{\n")
    w(f"{idt * 2}f.write_str(self.as_ref())\n")
    w(f"{idt}}}\n")
    w("}\n")


# TODO: use idt here for indentation
def generate_tags_localization_rs(
    locale: Locale, whitelisted_tags: list[WhitelistedTag], f
) -> None:
    w = f.write

    write_warning(f)

    w("use crate::lang::Lang;\n")
    w("use crate::models::yomitan::TagInfo;\n\n")

    w("pub const fn has_locale(lang: Lang) -> bool {\n")
    w("    matches!(lang, ")
    for i, iso in enumerate(locale):
        if i > 0:
            w(" | ")
        w(f"Lang::{iso.title()}")
    w(")\n")
    w("}\n\n")

    w(
        "pub fn localize_tag(lang: Lang, short_tag: &str) -> Option<(&'static str, &'static str)> {\n"
    )
    w("    match lang {\n")
    for iso in locale:
        w(f"        Lang::{iso.title()} => localize_tag_{iso}(short_tag),\n")
    w("        _ => None,\n")
    w("    }\n")
    w("}\n\n")

    # This is similar to get_tag_bank_as_tag_info but not enough to merge them.
    w("pub fn localize_tag_info(lang: Lang, tag_info: &mut TagInfo) {\n")
    w(
        "    if let Some((short, long)) = localize_tag(lang, tag_info.short_tag.as_str()) {\n"
    )
    w("        tag_info.short_tag = short.to_string();\n")
    w("        tag_info.long_tag = long.to_string();\n")
    w("    }\n")
    w("}\n")

    # Keyed by primary alias only. Secondary aliases are irrelevant.
    # This is because we only translate short tags. Every alias converges to a short tag
    # and it's that short tag that we will localize into: (trans.short_tag, trans.long_tag)
    long_to_short = {wt.long_tag(): wt.short_tag for wt in whitelisted_tags}

    for iso, translations in locale.items():
        ratio = len(translations) / len(long_to_short)

        w("\n")
        w(
            f"/// Coverage: {len(translations)}/{len(long_to_short)} tags ({ratio:.1%})\n"
        )
        w(
            f"fn localize_tag_{iso}(short_tag: &str) -> Option<(&'static str, &'static str)> {{\n"
        )
        w("    match short_tag {\n")
        for trans in translations:
            # SAFETY: we already checked that long_tag_en is in long_to_short
            short_key = long_to_short[trans.long_tag_en]
            # If the short tag was left empty, it means there is no short version
            # and we default to showing the long one.
            short_tag = trans.short_tag or trans.long_tag
            w(f'        "{short_key}" => Some(("{short_tag}", "{trans.long_tag}")),\n')
        w("        _ => None,\n")
        w("    }\n")
        w("}\n")


def load_lang(item: Any) -> Lang:
    return Lang(
        item["iso"],
        item["language"],
        item["displayName"],
        item["flag"],
        item.get("hasEdition", False),
    )


def load_langs(path: Path) -> list[Lang]:
    with path.open() as f:
        data = json.load(f)
    return [load_lang(item) for item in data]


def sort_languages_json(path: Path) -> None:
    with path.open() as f:
        text = f.read()

    lines = text.splitlines()
    langs = [
        (load_lang(json.loads(line.strip(","))), idx)
        for idx, line in enumerate(lines[1:-1])
    ]

    langs_sorted = sorted(langs, key=lambda pair: pair[0].display_name)
    if langs == langs_sorted:
        return

    with path.open("w") as f:
        f.write("[\n")
        for _, idx in langs_sorted:
            f.write(lines[idx + 1])
            f.write("\n")
        f.write("]\n")


def check_yomitan_langs(langs: list[Lang]) -> None:
    """Check if we support at least what is supported by yomitan.

    Since it sends a request, it is gated under the --check-yomitan flag.
    """
    import requests

    url = "https://raw.githubusercontent.com/yomidevs/yomitan/master/ext/js/language/language-descriptors.js"
    response = requests.get(url)
    response.raise_for_status()
    js_text = response.text

    # Get iso and names
    # ~ we assume that there is no inner lists [] in the descriptors.
    mch = re.search(r"const languageDescriptors\s*=\s*\[(.*?)\];", js_text, re.DOTALL)
    if not mch:
        print("Regex didn't match")
        return
    content = mch.group(1)

    # Quick and dirty regex to get iso/names
    iso_re = re.compile(r"iso: '(.*)',")
    name_re = re.compile(r"name: '(.*)',")
    isos = []
    names = []

    for line in content.splitlines():
        if iso_match := iso_re.search(line):
            isos.append(iso_match.group(1))
        if name_match := name_re.search(line):
            names.append(name_match.group(1))
    assert len(isos) == len(names)

    our_iso_map = {lang.iso: lang for lang in langs}
    missing_isos = []
    different_names = []

    for ymt_iso, ymt_name in zip(isos, names):
        if ymt_iso not in our_iso_map:
            # This iso is supported by yomitan but not us
            missing_iso = f"[missing iso] {ymt_iso} ({ymt_name})"
            missing_isos.append(missing_iso)
        else:
            our_lang = our_iso_map[ymt_iso]
            if ymt_name != our_lang.language:
                # For Arabic (and relatives), we have:
                # * yomitan: name='Arabic (MSA)'
                # * we:      language='Arabic', display_name='Arabic, MSA',
                #
                # In this case the name is different but it is fine.
                if ", " in our_lang.display_name:
                    main, variant = our_lang.display_name.split(", ")
                    rebuilt = f"{main} ({variant})"
                    if ymt_name == rebuilt:
                        continue

                # We have this iso, but the name is different
                different_name = f"[different name] {ymt_name=} but {our_lang=}"
                different_names.append(different_name)

    for logs, label in (
        (missing_isos, "missing_isos"),
        (different_names, "different_names"),
    ):
        if logs:
            for log in logs:
                print(log)
        else:
            print(f"✓ No {label}")


def check_kaikki_langs(langs: list[Lang]) -> None:
    """Check for unsupported (by us) languages in https://kaikki.org/dictionary/

    Since it sends a request, it is gated under the --check-kaikki flag.
    """
    import requests

    url = "https://kaikki.org/dictionary/"
    print(f"Checking for unsupported langs @ {url}")
    response = requests.get(url)
    response.raise_for_status()
    response.encoding = "utf-8"
    text = response.text

    supported = {lang.language for lang in langs}
    upto = 150

    # Get names (isos are not in the website)
    matches = re.findall(
        r"<li><a href=\"[^/]+/index\.html\">([^<]+) \((\d+) senses\)</a></li>",
        text,
        re.DOTALL,
    )

    for lang, num_senses in matches[:upto]:
        if lang in (
            "All languages combined",
            "Translingual",
            "Mandarin",  # We call it Chinese
        ):
            continue
        if lang not in supported:
            print(f"[missing from English kaikki ({upto})] {lang}, {num_senses}")


def sort_tags(tags: list[WhitelistedTag]) -> list[WhitelistedTag]:
    return sorted(
        tags,
        key=lambda wt: (
            wt.category == "",  # No category goes at the bottom
            wt.category,
            wt.sort_order,
            wt.short_tag,
        ),
    )


def load_sort_dump_tags(path: Path) -> list[WhitelistedTag]:
    with path.open() as f:
        data = json.load(f)
    tags = sort_tags([WhitelistedTag(*row) for row in data])
    # Overwrite to ensure formatting and sort
    with path.open("w") as f:
        json.dump([astuple(wt) for wt in tags], f, indent=4, ensure_ascii=False)
    return tags


def load_sort_dump_localization(
    path: Path, iso: str, long_to_wt: dict[str, WhitelistedTag]
) -> list[TagTranslation]:
    with path.open() as f:
        data = json.load(f)
    translations = [TagTranslation(k, v[0], v[1]) for k, v in data.items()]

    # SAFETY: no two long tags should be repeated
    seen_longs = set()
    for trans in translations:
        if trans.long_tag in seen_longs:
            print(f"[WARN] Duplicated long tag {trans.long_tag} for {iso}")
        seen_longs.add(trans.long_tag)

    # SAFETY: long_tag_en must be in long_to_short for sorting to work
    # (and, in general, it is most likely an error if it is not there)
    for trans in translations:
        if trans.long_tag_en not in long_to_wt:
            print(
                f"[{iso}] ERROR: tag '{trans.long_tag_en}' has no matching tag bank entry"
            )
            sys.exit(1)

    # Same sort logic as sort_tags
    translations = sorted(
        translations,
        key=lambda tr: (
            long_to_wt[tr.long_tag_en].category == "",
            long_to_wt[tr.long_tag_en].category,
            long_to_wt[tr.long_tag_en].sort_order,
            long_to_wt[tr.long_tag_en].short_tag,
        ),
    )

    # Overwrite to ensure formatting and sort
    with path.open("w") as f:
        json.dump(
            {tr.long_tag_en: [tr.short_tag, tr.long_tag] for tr in translations},
            f,
            indent=4,
            ensure_ascii=False,
        )

    return translations


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check-yomitan", action="store_true")
    parser.add_argument("--check-kaikki", action="store_true")
    args = parser.parse_args()
    check_yomitan = args.check_yomitan
    check_kaikki = args.check_kaikki

    src = Path("src")
    path_lang_rs = src / "lang.rs"
    path_tags_rs = src / "tags" / "tags_constants.rs"
    path_tags_loc_rs = src / "tags" / "tags_localization.rs"
    jsons_root = Path("assets")
    path_languages_json = jsons_root / "languages.json"
    path_tag_order_json = jsons_root / "tag_order.json"
    path_tag_bank_json = jsons_root / "tag_bank_term.json"
    path_tag_bank_variety_json = jsons_root / "tag_bank_term_variety.json"
    path_tag_locale_folder = jsons_root / "tags" / "locale"

    for path in (
        path_languages_json,
        path_tag_order_json,
        path_tag_bank_json,
        path_tag_bank_variety_json,
        path_tag_locale_folder,
    ):
        if not path.exists:
            print(f"Path does not exist @ {path}")
            return

    sort_languages_json(path_languages_json)

    langs = load_langs(path_languages_json)

    if check_kaikki:
        check_kaikki_langs(langs)

    if check_yomitan:
        check_yomitan_langs(langs)

    tag_order: list[str] = []
    with path_tag_order_json.open() as f:
        data = json.load(f)
        for _, tags in data.items():
            tag_order.extend(tags)
    # Overwrite to ensure formatting
    with path_tag_order_json.open("w") as f:
        json.dump(data, f, indent=4, ensure_ascii=False)

    whitelisted_tags = load_sort_dump_tags(path_tag_bank_json)
    for wt in whitelisted_tags:
        if wt.category == "variety":
            raise ValueError(f"{wt.short_tag}: 'variety' not allowed here")

    whitelisted_variety_tags = load_sort_dump_tags(path_tag_bank_variety_json)
    for wt in whitelisted_variety_tags:
        if wt.category != "variety":
            raise ValueError(f"{wt.short_tag}: expected 'variety', got '{wt.category}'")

    whitelisted_tags.extend(whitelisted_variety_tags)
    whitelisted_tags = sort_tags(whitelisted_tags)

    # read tags_X.json files inside localization folder, where X is the iso
    long_to_wt = {wt.long_tag(): wt for wt in whitelisted_tags}
    locale: Locale = {}
    for lang in langs:
        path_localization_json = path_tag_locale_folder / f"tags_{lang.iso}.json"
        if not path_localization_json.exists():
            continue
        locale[lang.iso] = load_sort_dump_localization(
            path_localization_json, lang.iso, long_to_wt
        )

    # import sys
    # generate_lang_rs(langs, sys.stdout)
    # generate_tags_rs(tag_order, sys.stdout)

    with path_lang_rs.open("w") as f:
        generate_lang_rs(langs, f)
        print(f"Wrote rust code @ {path_lang_rs}")
    with path_tags_rs.open("w") as f:
        generate_tags_rs(tag_order, whitelisted_tags, f)
        print(f"Wrote rust code @ {path_tags_rs}")
    with path_tags_loc_rs.open("w") as f:
        generate_tags_localization_rs(locale, whitelisted_tags, f)
        print(f"Wrote rust code @ {path_tags_loc_rs}")


if __name__ == "__main__":
    main()
