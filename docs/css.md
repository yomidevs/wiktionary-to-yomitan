## Prelude

- The css used by all `wty` dictionaries can be found [here](https://github.com/yomidevs/wiktionary-to-yomitan/blob/master/assets/styles/styles.css).
- Dictionaries made for non-Yomitan formats may also contain this simplified Yomitan-side [css](https://github.com/yomidevs/wiktionary-to-yomitan/blob/master/assets/styles/styles_yomitan.css).
- The original Yomitan-side css can be found at this [folder](https://github.com/yomidevs/yomitan/tree/master/ext/css), with secret css variables like `--text-color` at [material.css](https://github.com/yomidevs/yomitan/blob/master/ext/css/material.css). Note that these variables will not work in Anki because Yomitan does not support their extraction, and adding fallbacks in `styles.css` may interfere with the user theme.
- Yomitan documentation contains [themes](https://yomitan.wiki/tools-resources/?h=css#yomitan-colored-css) and how to set them up.
- The sunset Yomichan has a still mostly valid [guide](https://learnjapanese.moe/yomicss/) on how to customize css.

## Customize css

To add some custom css, follow the instructions from this [issue](https://github.com/yomidevs/wiktionary-to-yomitan/issues/244):

1. Open your Yomitan settings
2. Ensure `Advanced` is enabled at the bottom left
3. Select the `Appearance` section
4. Select the option `Configure custom CSS…`
5. In the `Popup CSS` section, paste the CSS code

---

The first thing to know, if you want the css to only affect a `wty` dictionary, is to preface every declaration with: 

```css
[data-dictionary="dictionary-name"]
```

For example, this will only show the "Hokkien" pronunciation in `wty-zh-en-ipa`:

```css
[data-dictionary="wty-zh-en-ipa"] .pronunciation {
    display: none;
}
[data-dictionary="wty-zh-en-ipa"] .pronunciation:has(.tag[data-details="Hokkien"]) {
    display: block;
}
```

!!! note "From now on, we will skip this declaration for simplicity."

## Main dictionary

### Sections

```css
/* Hide Grammar + Etymology sections on top */
div[data-sc-content="preamble"] {
  display: none;
}

/* Hide links at the bottom right */
div[data-sc-content="backlink"] {
  display: none;
}
```

### Tags

Here is a basic example on how to handle tags with your custom css.

```css
/* Hide inflections (~book icon, inflection tags) */
.inflection-rule-chains { display: none; }

/* Hide dictionary name (top-level tag) */
[data-category="dictionary"] { display: none; }
/* Hide gender tags (top-level tag) */
[data-category="gender"] { display: none; }

/* Hide inner topic tags (note the -sc-) */
[data-sc-category="topic"] { display: none; }
```

## Ipa dictionary

### Tags

This css will only show "Hokkien" and "bopomofo" pronunciations:

```css
.pronunciation {
    display: none;
}
.pronunciation:has(.tag[data-details="Hokkien"]) {
    display: block;
}
.pronunciation:has(.tag[data-details="bopomofo"]) {
    display: block;
}
```
