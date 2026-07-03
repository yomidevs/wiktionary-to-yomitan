//! Preprocesses word forms to only retain the headword.
//!
//! This amounts to fixing parsing errors in wiktextract and could be upstreamed.
//!
//! For example, in German, verb forms come with personal pronouns which makes for
//! poor results (worse search, deduplication, dictionary bloat etc.)

use crate::{
    lang::{Edition, Lang},
    models::kaikki::{Form, WordEntry},
};

pub fn preprocess_forms(edition: Edition, source: Lang, entry: &mut WordEntry) {
    match (edition, source, entry.pos.as_str()) {
        (Edition::De, Lang::De, "verb") => preprocess_verb_forms_de_de(entry),
        (Edition::De, Lang::De, "adj") => preprocess_adj_forms_de_de(entry),
        (Edition::De, Lang::De, "name") => preprocess_name_forms_de_de(entry),
        (Edition::De, Lang::De, "noun" | "phrase") => preprocess_noun_forms_de_de(entry),

        (Edition::En, Lang::Ga, _) => preprocess_forms_ga_en(entry),
        (Edition::Es, Lang::Es, "verb") => preprocess_verb_forms_es_es(entry),
        (Edition::Fr, Lang::Fr, "verb") => preprocess_verb_forms_fr_fr(entry),
        (Edition::It, Lang::It, "verb") => preprocess_verb_forms_it_it(entry),
        (Edition::Nl, Lang::Nl, "verb") => preprocess_verb_forms_nl_nl(entry),
        (Edition::Pt, Lang::Pt, "verb") => preprocess_verb_forms_pt_pt(entry),
        _ => (),
    }
}

fn strip_prefixes(entry: &mut WordEntry, prefixes: &[&str]) {
    debug_assert!(prefixes.iter().all(|pron| pron.ends_with(' ')));

    for form in &mut entry.forms {
        for &prefix in prefixes {
            if let Some(stripped) = form.form.strip_prefix(prefix) {
                form.form = stripped.to_string();
                break;
            }
        }
    }
}

fn contains_all(form: &Form, tags: &[&str]) -> bool {
    tags.iter()
        .all(|ctag| form.tags.iter().any(|tag| tag == ctag))
}

// See: https://kaikki.org/dewiktionary/Deutsch/meaning/a/au/ausmachen.html
fn preprocess_verb_forms_de_de(entry: &mut WordEntry) {
    // 1. Trim personal pronouns from verb forms (this information is already in tags)
    const PRONOUNS: &[&str] = &["ich ", "du ", "er/sie/es ", "wir ", "ihr ", "sie "];
    strip_prefixes(entry, PRONOUNS);

    // Another possible simplification:
    //
    // Reflexive forms, i.e. "wir meldeten uns an"
    // Just skip: there should be an active table anyway, and they become redundant.
    //
    // Here they have the reflexive tag:
    // https://de.wiktionary.org/wiki/Flexion:anmelden
    // ...but in general they don't need to, if there is only the reflexive option:
    // https://de.wiktionary.org/wiki/Flexion:fortscheren

    // 2. Remove auxiliary verb constructions
    //
    // Working with tags is better than doing string replacement, because in that case we may
    // stumble into edge cases like "anhaben", where we don't want to confuse the auxiliary verb
    // with the actual verb.
    // See: https://www.verblisten.de/listen/verben/anfangsbuchstabe/ueberblick.html?i=haben
    entry.forms.retain(|form| {
        let is_compound = form.tags.iter().any(|tag| {
            matches!(
                tag.as_str(),
                "perfect"
                    | "pluperfect"
                    | "future-i"
                    | "future-ii"
                    | "processual-passive"
                    | "statal-passive"
            )
        });

        !is_compound && !form.form.ends_with(['…', '!'])
            // "Partizip II des Verbs sehen, nur unmittelbar nach einem Infinitiv" etc.
            && !form.form.contains(',')
    });

    // The above tag strategy "requires"* us to clean the "extended" forms.
    // *I'm not entirely sure this is needed, specially because the form obtained is an adjective
    // that most likely happens in some other page, but since it gives the same result as the
    // (previous) string replacement, we keep it as it is.
    for form in &mut entry.forms {
        if let Some(stripped) = form.form.strip_prefix("zu ") {
            form.form = stripped.to_string();
        }
        if form.tags.iter().any(|tag| tag == "extended")
            && let Some(stripped) = form.form.strip_suffix(" zu haben")
        {
            form.form = stripped.to_string();
        }
    }
}

fn preprocess_adj_forms_de_de(entry: &mut WordEntry) {
    const PREFIXES: &[&str] = &["er ist ", "es ist ", "sie ist ", "sie sind "];
    strip_prefixes(entry, PREFIXES);
    strip_prefixes(entry, &["am "]);
}

// This was fixed for nouns in wiktextract, but I guess not names
fn preprocess_name_forms_de_de(entry: &mut WordEntry) {
    const PREFIXES: &[&str] = &[
        "des ", "(das) ", "dem ", "(dem) ", "der ", "(der) ", "die ", "(die) ", "den ",
    ];
    strip_prefixes(entry, PREFIXES);

    entry.forms.retain(|form| !form.form.ends_with('’'));
}

fn preprocess_noun_forms_de_de(entry: &mut WordEntry) {
    #[rustfmt::skip]
    const PREFIXES: &[&str] = &[
        // Nominative
        "der ", "das ", "die ",
        "ein ", "eine ", "keine ",
        // Accusative
        "den ", "einen ",
        // Dative
        "dem ", "einem ", "keinen ",
        // Genitive
        "des ", "eines ", "einer ", "keiner "
    ];
    strip_prefixes(entry, PREFIXES);
}

fn preprocess_forms_ga_en(entry: &mut WordEntry) {
    // https://en.wiktionary.org/wiki/crodh#Irish
    const PREFIXES: &[&str] = &["a ", "an ", "na ", "leis an ", "don ", "leis na "];
    strip_prefixes(entry, PREFIXES);
}

fn preprocess_verb_forms_es_es(entry: &mut WordEntry) {
    entry.forms.retain(|form| {
        let is_compound = form
            .tags
            .iter()
            .any(|tag| matches!(tag.as_str(), "pluperfect" | "compound"));
        let is_infinitive_impersonal = contains_all(form, &["infinitive", "impersonal"]);
        let is_perfect_subjunctive = contains_all(form, &["perfect", "subjunctive"]);

        !is_compound && !is_infinitive_impersonal && !is_perfect_subjunctive
    });
}

// This function is made based on preprocess_forms_de_de. See that function for more details.
fn preprocess_verb_forms_fr_fr(entry: &mut WordEntry) {
    const PRONOUNS: &[&str] = &[
        "je ",
        "j' ",
        "tu ",
        "il/elle/on ",
        "nous ",
        "vous ",
        "ils/elles ",
    ];
    strip_prefixes(entry, PRONOUNS);

    entry.forms.retain(|form| {
        let is_compound = form
            .tags
            .iter()
            .any(|tag| matches!(tag.as_str(), "perfect" | "pluperfect" | "anterior"));
        let is_past_conditional = contains_all(form, &["past", "conditional"]);
        let is_past_imperative = contains_all(form, &["past", "imperative"]);

        !is_compound && !is_past_conditional && !is_past_imperative
    });
}

fn preprocess_verb_forms_it_it(entry: &mut WordEntry) {
    const AVERE_AUX: &[&str] = &[
        "avrei ",
        "avresti ",
        "avrebbe ",
        "avremmo ",
        "avreste ",
        "avrebbero ",
        // These are actually perfect tense, but wiktionary doesn't parse them as such
        "abbia ",
        "abbiamo ",
        "abbiate ",
        "abbiano ",
        // Adding this here for simplicity: "non mangiare"
        "non ",
    ];
    strip_prefixes(entry, AVERE_AUX);

    entry.forms.retain(|form| {
        let is_compound = form
            .tags
            .iter()
            .any(|tag| matches!(tag.as_str(), "perfect" | "pluperfect" | "historic"));

        // mangiarsi (coniugazione)
        !is_compound && !form.form.ends_with(')')
    });
}

fn preprocess_verb_forms_nl_nl(entry: &mut WordEntry) {
    entry.forms.retain(|form| {
        // These are unusable in this shape...
        // * For the newlines, see https://github.com/tatuylonen/wiktextract/issues/1638
        // * "zou(dt) afnokken"
        //
        // Note that this was sort of fixed, and now there are a bunch of forms
        // with the auxiliary "zullen"
        let has_newline_or_parens = form.form.contains(['\n', '(']);

        let is_compound = form
            .tags
            .iter()
            .any(|tag| matches!(tag.as_str(), "perfect"));

        !has_newline_or_parens && !is_compound
    });
}

fn preprocess_verb_forms_pt_pt(entry: &mut WordEntry) {
    strip_prefixes(entry, &["não "]);
}
