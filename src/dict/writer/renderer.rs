//! HTML renderers for the YomitanEntry type.

use crate::models::yomitan::*;
use maud::{Markup, html};

pub trait Renderer {
    fn render_entry(entry: &YomitanEntry) -> Markup {
        match entry {
            YomitanEntry::TermBankEntry(t) => Self::render_term_bank_entry(t),
            YomitanEntry::TermBankEntryForm(t) => Self::render_term_bank_entry_form(t),
            YomitanEntry::TermMetaBankEntry(t) => Self::render_term_meta_bank_entry(t),
        }
    }

    fn render_term_bank_entry(entry: &TermBankEntry) -> Markup {
        html! {
            div class="entry" {
                div class="headword" {
                    span class="headword-term" {
                        (Self::render_headword(&entry.term, &entry.reading))
                    }
                }
                div class="entry-body" {
                    (Self::render_definition_tags(&entry.definition_tags))
                    @if entry.definitions.len() == 1 {
                        (Self::render_detailed_definition(&entry.definitions[0]))
                    } @else {
                        ol class="definition-list" {
                            @for def in &entry.definitions {
                                li {
                                    (Self::render_detailed_definition(def))
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn render_headword(term: &str, reading: &str) -> Markup {
        html!(
            ruby {
                (term)
                @if !reading.is_empty() {
                    rt { (reading) }
                }
            }
        )
    }

    fn render_definition_tags(tags: &[TagInfo]) -> Markup {
        html!(
            div class="definition-tag-list tag-list" {
                @for tag in tags {
                    (Self::render_definition_tag(tag))
                }
            }
        )
    }

    fn render_definition_tag(tag: &TagInfo) -> Markup {
        html!(
            span
                class="tag"
                title=(tag.long_tag)
                data-details=(tag.long_tag)
                data-category=(tag.category)
            {
                span class="tag-label" {
                    span class="tag-label-content" { (tag.short_tag) }
                }
            }
        )
    }

    fn render_term_bank_entry_form(entry: &TermBankEntryForm) -> Markup {
        html! {
            div class="entry form" {
                h2 { (&entry.term) }
                div class="reading" { (&entry.reading) }
                ul {
                    @for def in &entry.definitions {
                        li { (Self::render_detailed_definition(def)) }
                    }
                }
            }
        }
    }

    fn render_term_meta_bank_entry(entry: &TermMetaBankEntry) -> Markup {
        let TermMetaBankEntry::TermPhoneticTranscription(tm) = entry;
        html! {
            div class="entry form" {
                h2 { (tm.term) }
                h3 { (Self::render_phonetic_transcription(&tm.transcription)) }
            }
        }
    }

    fn render_phonetic_transcription(pt: &PhoneticTranscription) -> Markup {
        html! {
            b { (pt.reading) }
            ul {
                @for tr in &pt.transcriptions {
                    li { (tr.ipa) (tr.tags.join("|")) }
                }
            }
        }
    }

    fn render_detailed_definition(def: &DetailedDefinition) -> Markup {
        match def {
            DetailedDefinition::Text(s) => html! { (s) },
            DetailedDefinition::StructuredContent(s) => html! {
                span class="gloss-content structured-content" {
                    (Self::render_structured_content(s))
                }
            },
            DetailedDefinition::Inflection((label, forms)) => html! {
                b { (label) } ": " (forms.join(", "))
            },
        }
    }

    fn render_structured_content(sc: &StructuredContent) -> Markup {
        Self::render_node(&sc.content)
    }

    fn render_node(node: &Node) -> Markup {
        match node {
            Node::Text(t) => html! { (t) },
            Node::Array(nodes) => html! {
                @for n in nodes { (Self::render_node(n)) }
            },
            Node::Generic(g) => Self::render_generic_node(g),
            Node::Backlink(b) => Self::render_backlink(b),
        }
    }

    // Whether to skip rendering for this node or not.
    // This is the simplest way to skip some nodes for other formats.
    #[allow(unused_variables)]
    fn skip_render_generic_node(node: &GenericNode) -> bool {
        false
    }

    // This is way more convoluted that the pangloss version because maud requires
    // knowing the tag at compile time...
    fn render_generic_node(node: &GenericNode) -> Markup {
        if Self::skip_render_generic_node(node) {
            return html! {};
        }

        let content = Self::render_node(&node.content);

        let data = node.data.as_ref();
        let content_attr = data
            .and_then(|d| d.0.get(&NodeDataKey::Content))
            .map(|s| s.as_str());
        let category_attr = data
            .and_then(|d| d.0.get(&NodeDataKey::Category))
            .map(|s| s.as_str());
        let class = format!("gloss-sc-{}", node.tag.as_str());

        // https://github.com/lambda-fairy/maud/issues/240
        // The attr=[value] syntax skips the attribute if the value is None
        match node.tag {
            NTag::Span => html! {
                span
                    class=(class)
                    title=[node.title.clone()]
                    data-sc-content=[content_attr]
                    data-sc-category=[category_attr]
                { (content) }
            },
            NTag::Div => html! {
                div
                    class=(class)
                    data-sc-content=[content_attr]
                    data-sc-category=[category_attr]
                { (content) }
            },
            NTag::Ol => html! {
                ol
                    class=(class)
                    data-sc-content=[content_attr]
                    data-sc-category=[category_attr]
                { (content) }
            },
            NTag::Ul => html! {
                ul
                    class=(class)
                    data-sc-content=[content_attr]
                    data-sc-category=[category_attr]
                { (content) }
            },
            NTag::Li => html! {
                li
                    class=(class)
                    data-sc-content=[content_attr]
                    data-sc-category=[category_attr]
                { (content) }
            },
            NTag::Details => html! {
                details
                    class=(class)
                    data-sc-content=[content_attr]
                    data-sc-category=[category_attr]
                { (content) }
            },
            NTag::Summary => html! {
                summary
                    class=(class)
                    data-sc-content=[content_attr]
                    data-sc-category=[category_attr]
                { (content) }
            },
        }
    }

    fn render_backlink(b: &BacklinkContent) -> Markup {
        let label = match b.content {
            BacklinkContentKind::Wiktionary => "Wiktionary",
            BacklinkContentKind::Kaikki => "Kaikki",
        };
        html! {
            a href=(b.href) data-sc-content="backlink" { (label) }
        }
    }
}
