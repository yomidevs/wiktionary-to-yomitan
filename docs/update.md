## How to update a dictionary

wty dictionaries are updatable, so a new release can be fetched from Yomitan without reimporting anything:

1. Open Yomitan settings, go to "Dictionaries"
2. Click "Check for updates"
3. Click the `!` mark next to any dictionary that has one
4. Click "Update"

## Which dictionaries to import

The Yomitan [wiki](https://yomitan.wiki/dictionaries/) notes:

> Be aware that non-English dictionaries generally contain fewer entries than their English counterparts. Even if your primary language is not English, you may consider also importing the English version for better coverage.

This holds for wty too, because a dictionary can only contain what its Wiktionary edition wrote down. `wty main de es` is extracted from the Spanish edition, which documents far fewer German headwords than the English one does. Importing `wty main de en` alongside it fills those gaps, at the cost of English definitions.

## How updating works internally

A comprehensive guide about making yomitan dictionaries can be found [here](https://github.com/yomidevs/yomitan/blob/master/docs/making-yomitan-dictionaries.md).

Updating is done via the dictionary index ([schema](https://github.com/yomidevs/yomitan/blob/master/ext/data/schemas/dictionary-index-schema.json)), and more precisely, via these four attributes:

1. `revision`: the semantic or calendar version of the dictionary. We use calendar for all dictionaries.
2. `isUpdatable`: set to true, makes the dictionary updatable.
3. `indexUrl`: points to an unzipped copy of the new dictionary index.
4. `downloadUrl`: points to a zipped version of the new dictionary.

When clicking `Check for Updates`, the [yomitan code](https://github.com/yomidevs/yomitan/blob/c0abb9e98a15aeb6b6f8f6e2d91fe5e54240b54a/ext/js/dictionary/dictionary-data-util.js#L350) compares the `revision` of the current, imported dictionary, with the one in the unzipped index at `indexUrl`. If the revision found in the latter is more recent, it downloads from `downloadUrl` the new dictionary and replaces the old version.

Example:

```json
{
  "revision": "2026.02.22",
  "isUpdatable": true,
  "indexUrl": "https://some/website/wty-el-el-index.json",
  "downloadUrl": "https://some/website/wty-el-el.zip",
  ...
}
```
