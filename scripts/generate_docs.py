"""Generate a single static downloads page with dropdowns."""

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any

REPO_ID = "daxida/wty-release"
REPO_URL = f"https://huggingface.co/datasets/{REPO_ID}"

# duplicated from build
@dataclass
class Lang:
    iso: str
    language: str
    display_name: str
    flag: str
    # https://github.com/tatuylonen/wiktextract/tree/master/src/wiktextract/extractor
    has_edition: bool


# duplicated from build
def load_lang(item: Any) -> Lang:
    return Lang(
        item["iso"],
        item["language"],
        item["displayName"],
        item["flag"],
        item.get("hasEdition", False),
    )


# duplicated from build
def load_langs(path: Path) -> list[Lang]:
    with path.open() as f:
        data = json.load(f)
    return [load_lang(item) for item in data]


# (value, label) of the formats with a release_metadata_<format>.json
FORMATS = [("yomitan", "Yomitan"), ("mdict", "Mdict")]


def render_dropdown_options(options: list[tuple[str, str]]) -> str:
    return "\n".join(
        f'  <option value="{value}">{label}</option>' for value, label in options
    ).strip()


def render_combobox(cl: str, placeholder: str, options: list[tuple[str, str]]) -> str:
    assert cl in ("dl-source", "dl-target", "dl-format")
    return f"""
<div class="{cl}-combobox">
  <input class="{cl}-search" placeholder="{placeholder}" autocomplete="off">
  <div class="{cl}-dropdown">
    {render_dropdown_options(options)}
  </div>
  <input type="hidden" class="{cl}">
</div>""".strip()


def generate_downloads_page(all_langs: list[Lang]) -> str:
    langs = [(lang.iso, lang.display_name) for lang in all_langs]
    table_html = f"""
<tr>
  <td>{render_combobox("dl-source", "Source: language of the word...", langs)}</td>
  <td>{render_combobox("dl-target", "Target: language of the definition...", langs)}</td>
  <td>{render_combobox("dl-format", "Format...", FORMATS)}</td>
</tr>""".strip()

    return f"""# Download

<table class="download-table">
  <tbody>
{table_html}
  </tbody>
</table>

<div class="dl-results"></div>

!!! tip "You can import a dictionary directly to Yomitan by pasting the URL into "Import from URLs" (requires **Advanced** settings)"

Files are hosted [here]({
        REPO_URL
    }), where you can also see the calendar version (calver) of the dictionaries.

A brief description of the dictionaries can be found [here](dictionaries.md).
""".strip()


def generate_language_page(all_langs, editions) -> str:
    return f"""
[Kaikki](https://kaikki.org/) currently supports **{len(editions)} Wiktionary editions**. Every dictionary uses at least one edition.

For a list of **targets** supported by the English edition, see [here](https://kaikki.org/dictionary/).

For a list of supported languages by Yomitan, see [here](https://yomitan.wiki/supported-languages/). If it is outdated, refer to [here](https://raw.githubusercontent.com/yomidevs/yomitan/master/ext/js/language/language-descriptors.js).

If a language supported by us is not supported by Yomitan, you can add basic Yomitan support by submitting a pull request to Yomitan, similar to [this](https://github.com/yomidevs/yomitan/commit/c0abb9e98a15aeb6b6f8f6e2d91fe5e54240b54a). A more detailed guide can be found [here](https://github.com/yomidevs/yomitan/blob/master/docs/development/language-features.md).

`Simple English` is made from the [Simple English Wiktionary](https://simple.wiktionary.org/wiki/Main_Page), and contains only English glosses for English words. It is referred to by the made-up iso `simple`, and can only be used as a monolingual main or IPA dictionary.

!!! tip "Missing a language? Please **open an [issue](https://github.com/yomidevs/wiktionary-to-yomitan/issues/new)**."

---

**With Wiktionary Editions ({len(editions)}):**  
{", ".join(f"{edition.flag} {edition.display_name} `{edition.iso}`" for edition in editions)}

**All Supported ({len(all_langs)}):**  
{", ".join(f"{lang.flag} {lang.display_name} `{lang.iso}`" for lang in all_langs)}
""".strip()


def main() -> None:
    path_language_json = Path("assets/languages.json")
    path_docs = Path("docs")
    path_download = path_docs / "download.md"
    path_language = path_docs / "language.md"

    print("Loading languages...")
    all_langs = load_langs(path_language_json)
    editions = [lang for lang in all_langs if lang.has_edition]

    print(f"Found {len(all_langs)} languages, {len(editions)} with edition")

    print(f"Generating downloads page @ {path_download}")
    path_download.write_text(generate_downloads_page(all_langs))

    print(f"Generating language page @ {path_language}")
    path_language.write_text(generate_language_page(all_langs, editions))

    print("✓ Done!")


if __name__ == "__main__":
    main()
