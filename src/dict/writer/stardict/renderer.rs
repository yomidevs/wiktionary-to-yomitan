//! Rendering for the Stardict format that mostly targets KOReader.
//!
//! Note that, as of now, ruby is not supported, nor are links.

use maud::{Markup, html};

use crate::{
    dict::writer::renderer::Renderer,
    models::yomitan::{BacklinkContent, GenericNode, NTag, NodeDataKey, TagInfo},
};

// Rendering that mostly targets KOReader
pub struct StardictRenderer;

impl Renderer for StardictRenderer {
    fn skip_render_generic_node(node: &GenericNode) -> bool {
        if matches!(node.tag, NTag::Details) {
            return true;
        }
        let data = node.data.as_ref();
        let content_attr = data
            .and_then(|d| d.0.get(&NodeDataKey::Content))
            .map(|s| s.as_str());
        matches!(
            content_attr,
            Some("preamble" | "summary-entry" | "example-sentence")
        )
    }

    // Render headword without ruby
    fn render_headword(term: &str, reading: &str) -> Markup {
        html!(
            (term)
            @if !reading.is_empty() {
                span class="reading" { " [" (reading) "]" }
            }
        )
    }

    fn render_definition_tags(tags: &[TagInfo]) -> Markup {
        html! {
            h3 {
                @for (i, tag) in tags.iter().enumerate() {
                    @if i > 0 { " · " }
                    (tag.long_tag)
                }
            }
        }
    }

    fn render_backlink(_: &BacklinkContent) -> Markup {
        html! {}
    }
}
