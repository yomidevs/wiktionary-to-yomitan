Dictionaries are made from extracted wiktionary content. Their quality depends on wiktionary [coverage](https://meta.wikimedia.org/wiki/Wiktionary/Table) and [wiktextract](https://github.com/tatuylonen/wiktextract) quality of extracted fields.

## Dictionary types

These are the different types of dictionaries that can be made with wty (optional parameters have been omitted):

```console
$ wty main              <SOURCE> <TARGET>
$ wty ipa               <SOURCE> <TARGET>
$ wty ipa-merged        <TARGET>
$ wty glossary          <SOURCE> <TARGET>
$ wty glossary-extended <EDITION> <SOURCE> <TARGET>
```

- **main**: main dictionaries, with etymology, examples etc. These have good coverage, but tend to be verbose.
- **glossary**: short dictionaries made from Wiktionary translations section.
- **ipa**: pronunciation dictionaries.

!!! tip "Reminder: roughly, the source is the language we learn. The target is the language we know."

| Dictionary type | Edition(s)  | Source  | Target  |
| --------------- | -------- | ------- | ------- |
| **main**        | **TARGET** | source  | **TARGET** |
| **ipa**         | **TARGET** | source  | **TARGET** |
| **ipa-merged**  | ALL    | X    | target |
| **glossary**    | **SOURCE** | **SOURCE** | target |
| **glossary-extended**    | edition | source | target |

!!! tip "Identical cells in a row are highlighted in bold UPPERCASE"

## Paths

When building locally, dictionaries are usually stored in: `ROOT/dict/SOURCE/TARGET/wty-SOURCE-TARGET.zip`.

The only exception being ipa-merged, since it has no source.

```console
$ wty main de en
✓ Wrote yomitan dict @ data/dict/de/en/wty-de-en.zip (16.05 MB)
$ wty glossary de en
✓ Wrote yomitan dict @ data/dict/de/en/wty-de-en-gloss.zip (3.58 MB)
$ wty ipa-merged en
✓ Wrote yomitan dict @ data/dict/all/en/wty-en-ipa.zip (4.45 MB)
$ wty glossary-extended all de en
✓ Wrote yomitan dict @ data/dict/de/en/wty-all-de-en-gloss.zip (2.70 MB)
```

## Notes

As of now, there is no way to make a **main** dictionary with only lemmas, or only forms. See this [issue](https://github.com/yomidevs/wiktionary-to-yomitan/issues/166).

It is possible to hack your way around it, either by modifying the code as stated in the issue above, or by manually deleting the unwanted banks,
since the writer jumps to a new bank when it goes from lemmas to forms.

