use crate::{dict::rules::valid::is_valid_rule, lang::Lang};

// Note that short_tags always includes the pos in the main dict
pub fn rule_identifiers(source: Lang, short_tags: &[String]) -> String {
    tags_to_rules(source, short_tags).join(" ")
}

fn replace_rule<'a>(rules: &mut Vec<&'a str>, remove: &str, add: &'a str) {
    rules.retain(|r| *r != remove);
    rules.push(add);
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
                    replace_rule(&mut rules, "n", "ns");
                }
                if short_tags.iter().any(|t| t == "pl") {
                    replace_rule(&mut rules, "n", "np");
                }
            }
        }
        Lang::Ja => {
            for tag in short_tags {
                match tag.as_str() {
                    "ichidan" => replace_rule(&mut rules, "v", "v1"),
                    "godan" => replace_rule(&mut rules, "v", "v5"),
                    "sa-row" => replace_rule(&mut rules, "v", "vs"),
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
