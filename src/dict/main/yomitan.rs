//! How to convert the intermediate representation into a Vec<[`YomitanEntry`]>.

use crate::{
    Set,
    cli::LangSpecs,
    dict::main::{
        ir::{FormMap, GlossTree, LemmaInfo, LemmaMap, Tidy, normalize_orthography},
        locale::{
            localize_etymology_string, localize_examples_string, localize_grammar_string,
            localize_synonyms_string,
        },
    },
    lang::Lang,
    models::{
        kaikki::{Example, Offset, Synonym, Tag},
        yomitan::{
            BacklinkContent, BacklinkContentKind, DetailedDefinition, GenericNode, NTag, Node,
            NodeData, NodeDataKey, TagInfo, TermBankEntry, TermBankEntryForm, YomitanDict, wrap,
        },
    },
    tags::{Pos, find_tag_in_bank, localize_tag, localize_tag_info},
};

pub fn to_yomitan_impl(langs: LangSpecs, irs: &Tidy) -> YomitanDict {
    YomitanDict::new(
        to_yomitan_lemmas(langs.target, &irs.lemma_map),
        to_yomitan_forms(langs.source, &irs.form_map),
        vec![],
    )
}

#[tracing::instrument(skip_all, level = "trace")]
fn to_yomitan_lemmas(target: Lang, lemma_map: &LemmaMap) -> Vec<TermBankEntry> {
    lemma_map
        .flat_iter()
        .map(move |(lemma, reading, pos, info)| to_yomitan_lemma(target, lemma, reading, pos, info))
        .collect()
}

fn to_yomitan_lemma(
    target: Lang,
    lemma: &str,
    reading: &str,
    pos: Pos,
    info: &LemmaInfo,
) -> TermBankEntry {
    let short_pos = pos.short();

    let common_tag_infos_found = get_found_tags(pos, info);
    let common_short_tags_found: Vec<_> = common_tag_infos_found
        .iter()
        .map(|tag_info| tag_info.short_tag.clone())
        .collect();
    let definition_tags: Vec<_> = common_tag_infos_found
        .into_iter()
        .map(|mut tag_info| {
            localize_tag_info(target, &mut tag_info);
            tag_info
        })
        .collect();

    let mut detailed_definition_content = Node::new_array();

    if info.etymology_text.is_some() || info.head_info_text.is_some() {
        detailed_definition_content.push(structured_preamble(
            target,
            info.etymology_text.clone(),
            info.head_info_text.clone(),
        ));
    }

    detailed_definition_content.push(structured_glosses(
        target,
        info.gloss_tree.clone(),
        &common_short_tags_found,
    ));

    if let Some(synonyms_node) = structured_synonyms(target, &info.synonyms) {
        detailed_definition_content.push(synonyms_node);
    }

    detailed_definition_content.push(structured_backlink(
        info.link_wiktionary.clone(),
        info.link_kaikki.clone(),
    ));

    TermBankEntry::new(
        lemma.to_string(),
        reading.to_string(),
        definition_tags,
        get_rule_identifier(short_pos),
        vec![DetailedDefinition::structured(detailed_definition_content)],
    )
}

/// Extracts and normalizes tags associated with a lemma.
///
/// This function collects tags from three sources:
/// 1. The provided part-of-speech (`pos`) - always included first
/// 2. Top-level tags from `LemmaInfo` (`info.tags`) - usu. non-En edition
/// 3. Tags common to all senses in `info.gloss_tree` - usu. En edition
///
/// For sense-level tags, only those present in *every* gloss are kept
/// (set intersection across all gloss entries).
fn get_found_tags(pos: Pos, info: &LemmaInfo) -> Vec<TagInfo> {
    let common_tags_iter = info
        .gloss_tree
        .values()
        .map(|g| Set::from_iter(g.tags.iter().map(String::as_str)))
        .reduce(|acc, set| acc.intersection(&set).copied().collect::<Set<&str>>())
        .unwrap() // a non-empty gloss_tree has at least one gloss
        .into_iter();

    std::iter::once(pos.long())
        .chain(info.tags.iter().map(String::as_str)) // top level tags (the non-En preferred way)
        .chain(common_tags_iter)
        .filter_map(|tag| find_tag_in_bank(tag))
        .collect()
}

// There could be multiple identifiers, but let's start with one.
//
// This function is trivial at the moment, but could be worked on to validate identifiers,
// add multiple identifiers, merge tags into more useful identifiers (verb: v, transitive: t > vt),
// remove unused identifiers etc.
fn get_rule_identifier(short_pos: &str) -> String {
    short_pos.to_string()
}

fn build_details_entry(ty: &str, ty_loc: &str, content: String) -> Node {
    wrap(
        NTag::Details,
        &format!("details-entry-{ty}"),
        Node::Array(vec![
            wrap(NTag::Summary, "summary-entry", Node::Text(ty_loc.into())),
            wrap(NTag::Div, &format!("{ty}-content"), Node::Text(content)),
        ]),
    )
}

fn structured_preamble(
    target: Lang,
    etymology_text: Option<String>,
    head_info_text: Option<String>,
) -> Node {
    let mut preamble_content = Node::new_array();
    if let Some(head_info_text) = head_info_text {
        let ty_loc = localize_grammar_string(target);
        preamble_content.push(build_details_entry("Grammar", ty_loc, head_info_text));
    }
    if let Some(etymology_text) = etymology_text {
        let ty_loc = localize_etymology_string(target);
        preamble_content.push(build_details_entry("Etymology", ty_loc, etymology_text));
    }

    wrap(
        NTag::Div,
        "",
        wrap(NTag::Div, "preamble", preamble_content).into_array_node(),
    )
}

fn structured_backlink(wlink: String, klink: String) -> Node {
    wrap(
        NTag::Div,
        "backlink",
        Node::Array(vec![
            Node::Backlink(BacklinkContent::new(wlink, BacklinkContentKind::Wiktionary)),
            Node::Text(" | ".into()), // JMdict uses this separator
            Node::Backlink(BacklinkContent::new(klink, BacklinkContentKind::Kaikki)),
        ]),
    )
}

fn structured_glosses(
    target: Lang,
    gloss_tree: GlossTree,
    common_short_tags_found: &[Tag],
) -> Node {
    wrap(
        NTag::Ol,
        "glosses",
        Node::Array(
            gloss_tree
                .into_iter()
                .map(|gloss_pair| {
                    wrap(
                        NTag::Li,
                        "",
                        Node::Array(structured_glosses_go(
                            target,
                            &GlossTree::from_iter([gloss_pair]),
                            common_short_tags_found,
                            0,
                        )),
                    )
                })
                .collect(),
        ),
    )
}

// Recursive helper ~ should return Node for consistency
fn structured_glosses_go(
    target: Lang,
    gloss_tree: &GlossTree,
    common_short_tags_found: &[Tag],
    level: usize,
) -> Vec<Node> {
    let html_tag = if level == 0 { NTag::Div } else { NTag::Li };
    let mut nested = Vec::new();

    for (gloss, gloss_info) in gloss_tree {
        // Tags/topics that are not common to all glosses (i.e. specific to this gloss)
        let minimal_tags: Vec<_> = gloss_info
            .tags
            .iter()
            .chain(gloss_info.topics.iter())
            .filter(|&tag| !common_short_tags_found.contains(tag))
            .cloned()
            .collect();

        let mut level_content = Node::new_array();

        if let Some(structured_tags) =
            structured_tags(target, &minimal_tags, common_short_tags_found)
        {
            level_content.push(structured_tags);
        }

        level_content.push(Node::Text(gloss.into()));

        if !gloss_info.examples.is_empty() {
            level_content.push(structured_examples(target, &gloss_info.examples));
        }

        nested.push(wrap(html_tag, "", level_content));

        let Some(children) = &gloss_info.children else {
            continue;
        };

        // We dont want tags from the parent appearing again in the children
        let mut new_common_short_tags_found = minimal_tags;
        new_common_short_tags_found.extend_from_slice(common_short_tags_found);

        nested.push(wrap(
            NTag::Ul,
            "",
            Node::Array(structured_glosses_go(
                target,
                children,
                &new_common_short_tags_found,
                level + 1,
            )),
        ));
    }

    nested
}

/// Structure inner tags.
///
/// We sort them ourselves since yomitan only sorts top-level tags.
///
/// cf [`crate::models::yomitan::TagInfo`]
fn structured_tags(target: Lang, tags: &[Tag], common_short_tags_found: &[Tag]) -> Option<Node> {
    let mut tag_infos: Vec<_> = tags
        .iter()
        .filter_map(|tag| {
            let tag_info = find_tag_in_bank(tag)?;
            if common_short_tags_found.contains(&tag_info.short_tag) {
                None
            } else {
                Some(tag_info)
            }
        })
        .collect();

    tag_infos.sort_unstable_by_key(|t| t.sort_order);

    let structured_tags_content: Vec<_> = tag_infos
        .into_iter()
        .map(|tag_info| {
            let (short_tag, long_tag) = match localize_tag(target, &tag_info.short_tag) {
                Some((short, long)) => (short.to_string(), long.to_string()),
                None => (tag_info.short_tag, tag_info.long_tag),
            };
            GenericNode {
                tag: NTag::Span,
                title: Some(long_tag),
                data: Some(NodeData::from_iter([
                    (NodeDataKey::Content, "tag"),
                    (NodeDataKey::Category, &tag_info.category),
                ])),
                content: Node::Text(short_tag),
            }
            .into_node()
        })
        .collect();

    if structured_tags_content.is_empty() {
        None
    } else {
        Some(wrap(
            NTag::Div,
            "tags",
            Node::Array(structured_tags_content),
        ))
    }
}

fn structured_examples(target: Lang, examples: &[Example]) -> Node {
    debug_assert!(!examples.is_empty());

    let localized_label = wrap(
        NTag::Summary,
        "summary-entry",
        Node::Text(localize_examples_string(target, examples.len())),
    );

    wrap(
        NTag::Details,
        "details-entry-examples",
        Node::Array(
            std::iter::once(localized_label)
                .chain(examples.iter().map(structured_example))
                .collect(),
        ),
    )
}

// TODO: change a-b-c into a more descriptive name: text/translation/ref
fn structured_example(example: &Example) -> Node {
    let mut structured_example_content = wrap(
        NTag::Div,
        "example-sentence-a",
        structured_example_text(&example.text, &example.bold_text_offsets),
    )
    .into_array_node();

    if !example.translation.is_empty() {
        structured_example_content.push(wrap(
            NTag::Div,
            "example-sentence-b",
            structured_example_text(&example.translation, &example.bold_translation_offsets),
        ));
    }

    if !example.reference.is_empty() {
        let reference = example
            .reference
            .strip_suffix(':')
            .unwrap_or(&example.reference)
            .to_string();
        structured_example_content.push(wrap(
            NTag::Div,
            "example-sentence-c",
            Node::Text(reference),
        ));
    }

    wrap(
        NTag::Div,
        "extra-info",
        wrap(NTag::Div, "example-sentence", structured_example_content),
    )
}

/// Wraps in `NTag::Span` bold ranges if there are any.
///
/// Note that wiktextract only extracts bold offsets for Examples.
fn structured_example_text(text: &str, offsets: &[Offset]) -> Node {
    if offsets.is_empty() {
        return Node::Text(text.to_string());
    }

    let chars: Vec<_> = text.chars().collect();
    let upto = chars.len();

    let offsets = sanitize_offsets(offsets, upto);
    if offsets.is_empty() {
        return Node::Text(text.to_string());
    }

    let mut content = Node::new_array();
    let mut last = 0;

    for (start, end) in offsets {
        // Push what comes before the bold offset
        if last < start {
            content.push(Node::Text(chars[last..start].iter().collect()));
        }

        // Push the bold offset
        content.push(wrap(
            NTag::Span,
            "bold-text",
            Node::Text(chars[start..end].iter().collect()),
        ));

        last = end;
    }

    if last < chars.len() {
        content.push(Node::Text(chars[last..].iter().collect()));
    }

    content
}

fn structured_synonyms(target: Lang, synonyms: &[Synonym]) -> Option<Node> {
    if synonyms.is_empty() {
        return None;
    }

    Some(wrap(
        NTag::Div,
        "synonyms",
        Node::Array(vec![
            wrap(
                NTag::Div,
                "synonyms-label",
                Node::Text(localize_synonyms_string(target).into()),
            ),
            wrap(
                NTag::Ul,
                "synonyms-list",
                Node::Array(
                    synonyms
                        .iter()
                        .map(|syn| wrap(NTag::Li, "synonym-item", Node::Text(syn.word.clone())))
                        .collect(),
                ),
            ),
        ]),
    ))
}

/// Returns only valid, non-overlapping offsets within bounds of `upto`.
///
/// We ASSUME that offsets are sorted by start. That is, given [O1, O2, ...] with
/// O1 = (a, b) and O2 = (c, d), we assume a <= c etc.
///
/// Merges offsets with a non-trivial intersection. That is, if O1 and O2 overlap, we merge into
/// O3 = (a, max(b, d)).
fn sanitize_offsets(offsets: &[Offset], upto: usize) -> Vec<Offset> {
    let mut sanitized: Vec<Offset> = Vec::new();
    for &(start, end) in offsets {
        debug_assert!(start < end);
        if end > upto {
            // Out of bound offset: skip
            continue;
        }
        match sanitized.last_mut() {
            Some(prev) if start < prev.1 => prev.1 = prev.1.max(end),
            _ => sanitized.push((start, end)),
        }
    }
    sanitized
}

#[tracing::instrument(skip_all, level = "trace")]
fn to_yomitan_forms(source: Lang, form_map: &FormMap) -> Vec<TermBankEntryForm> {
    form_map
        .flat_iter()
        .map(move |(uninflected, inflected, pos, _, tags)| {
            // There needs to be DetailedDefinition per tag because yomitan reads
            // multiple tags in a single Inflection as a causal inflection chain.
            let deinflection_definitions: Vec<_> = tags
                .iter()
                .map(|tag| {
                    DetailedDefinition::Inflection((uninflected.to_string(), vec![tag.clone()]))
                })
                .collect();

            let normalized_inflected = normalize_orthography(source, inflected);
            let reading = if normalized_inflected == inflected {
                String::new()
            } else {
                inflected.to_string()
            };

            let short_pos = pos.short();

            TermBankEntryForm::new(
                normalized_inflected,
                reading,
                get_rule_identifier(short_pos),
                deinflection_definitions,
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::sanitize_offsets;

    #[test]
    fn offset_base() {
        assert_eq!(sanitize_offsets(&[], 10), vec![]);
        assert_eq!(
            sanitize_offsets(&[(0, 2), (3, 5)], 10),
            vec![(0, 2), (3, 5)]
        );
    }

    #[test]
    fn offset_out_of_bounds() {
        assert_eq!(sanitize_offsets(&[(0, 2), (3, 20)], 10), vec![(0, 2)]);
    }

    #[test]
    fn offset_overlap() {
        // 1. b == c, treated as non-overlapping
        assert_eq!(
            sanitize_offsets(&[(0, 2), (2, 4)], 10),
            vec![(0, 2), (2, 4)]
        );

        // 2. a == c, keep larger
        assert_eq!(sanitize_offsets(&[(10, 12), (10, 13)], 20), vec![(10, 13)]);

        // 3. b == d, keep larger
        assert_eq!(sanitize_offsets(&[(10, 12), (11, 12)], 20), vec![(10, 12)]);

        // 4. Inner is smaller - discard it
        assert_eq!(sanitize_offsets(&[(0, 10), (2, 5)], 20), vec![(0, 10)]);

        // 5. a < c < b < d - merge into (a, d)
        assert_eq!(sanitize_offsets(&[(0, 5), (3, 8)], 20), vec![(0, 8)]);
    }
}
