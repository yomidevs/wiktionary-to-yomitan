//! Tags of kaikki examples.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// The script of an example, as a `lang` attribute, so that users can hide one with CSS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub enum ScriptTag {
    ZhHant,
    ZhHans,
}

impl ScriptTag {
    /// Examples tagged with both scripts (i.e. identical in both) have none.
    pub fn deserialize_kaikki_tags<'de, D: Deserializer<'de>>(
        d: D,
    ) -> Result<Option<Self>, D::Error> {
        let tags = Vec::<String>::deserialize(d)?;
        let has = |tag| tags.iter().any(|t| t == tag);
        Ok(
            match (has("Traditional-Chinese"), has("Simplified-Chinese")) {
                (true, false) => Some(Self::ZhHant),
                (false, true) => Some(Self::ZhHans),
                _ => None,
            },
        )
    }

    pub const fn as_lang_attr(self) -> &'static str {
        match self {
            Self::ZhHant => "zh-Hant",
            Self::ZhHans => "zh-Hans",
        }
    }
}

impl Serialize for ScriptTag {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_lang_attr())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn example_lang(json: &str) -> Option<ScriptTag> {
        serde_json::from_str::<crate::models::kaikki::Example>(json)
            .unwrap()
            .lang
    }

    #[test]
    fn script_tag_from_example_tags() {
        let trad = r#"{"text": "x", "tags": ["Traditional-Chinese", "Pinyin"]}"#;
        let simp = r#"{"text": "x", "tags": ["Pinyin", "Simplified-Chinese"]}"#;
        let both = r#"{"text": "x", "tags": ["Traditional-Chinese", "Simplified-Chinese"]}"#;
        let missing = r#"{"text": "x"}"#;
        assert_eq!(example_lang(trad), Some(ScriptTag::ZhHant));
        assert_eq!(example_lang(simp), Some(ScriptTag::ZhHans));
        assert_eq!(example_lang(both), None);
        assert_eq!(example_lang(missing), None);
    }
}
