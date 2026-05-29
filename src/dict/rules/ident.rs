use crate::{dict::rules::valid::is_valid_rule, lang::Lang};

// Note that short_tags always includes the pos in the main dict
pub fn rule_identifiers(source: Lang, short_tags: &[String]) -> String {
    tags_to_rules(source, short_tags).join(" ")
}

fn tags_to_rules<'a>(source: Lang, short_tags: &'a [String]) -> Vec<&'a str> {
    let mut rules: Vec<_> = short_tags
        .iter()
        .filter_map(|tag| is_valid_rule(source, tag).then_some(tag.as_str()))
        .collect();

    match source {
        Lang::Es => {
            if rules.contains(&"n") {
                if short_tags.iter().any(|t| t == "sg") {
                    rules.push("ns");
                }
                if short_tags.iter().any(|t| t == "pl") {
                    rules.push("np");
                }
            }
        }
        Lang::Ja => {
            for tag in short_tags {
                match tag.as_str() {
                    "ichidan" => rules.push("v1"),
                    "godan" => rules.push("v5"),
                    // "adj" => rules.push("adj-i"), // we don't know
                    "sa-row" => rules.push("vs"),
                    _ => {}
                }
            }
        }
        _ => {}
    }

    // WARN: this shouldn't be in release
    if let Some(invalid) = rules.iter().find(|r| !is_valid_rule(source, r)) {
        panic!("Found invalid rule for {source}: {invalid}")
    };

    rules
}
