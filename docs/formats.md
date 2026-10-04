# Formats

The full list of available formats can be seen in the CLI passing the `--help` flag.

| Format | Requires extra tools | Downloads | Used in |
|--------|----------------------|-----------|---------|
| `yomitan` | ❌ | ✅ | Yomitan |
| `mdict` | ❌ | ✅ | GoldenDict-ng |
| `stardict` | ❌ | ❌ | KOReader |

The remaining values (`html`, `ir`, `debug-forms`, `skip`) are debugging aids.

To make a dictionary in a certain format:

```console
# Defaults to yomitan
$ wty main ja en

# Glossary dictionary in mdict
$ wty glossary de en --format=mdict

# Ipa dictionary in stardict
$ wty ipa el el --format=stardict

# Several formats at once (the dump is read once)
$ wty main ja en --format=yomitan,mdict
```

---

## `yomitan` *(default)*

Produces a zip archive importable into yomitan. Downloads are available [here](download.md).

## `mdict`

Produces a `*.mdx` file and a `*.mdd` file holding the css, that can be imported as is. It is written with [pangloss](https://github.com/daxida/pangloss), so no external MDict conversion tool is needed. Downloads are available [here](download.md) as a zip with both files: unzip it and import the folder.

## `stardict`

Produces a folder with `*.ifo`, `*.dict` etc. files that can be directly imported.

KOReader dictionary install [guide](https://github.com/koreader/koreader/wiki/Dictionary-support).
