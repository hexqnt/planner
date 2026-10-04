use crate::{
    model::{EventLink, InputError},
    text::Language,
};

use super::{ParsedTextInput, TextDraft, TextKind};

pub(in crate::app) struct LinkKind;
pub(in crate::app) type LinkDraft = TextDraft<LinkKind>;
pub(in crate::app) type LinkInput<'a> = ParsedTextInput<'a, LinkKind>;

impl LinkDraft {
    pub(in crate::app) fn from_link(link: Option<&EventLink>) -> Self {
        Self::from_value(link.cloned())
    }
}

impl TextKind for LinkKind {
    type Value = Option<EventLink>;
    const EMPTY: Self::Value = None;
    const MULTILINE: bool = false;

    fn parse(text: &str) -> Result<Self::Value, InputError> {
        let text = text.trim();
        if text.is_empty() {
            Ok(None)
        } else {
            text.parse().map(Some)
        }
    }

    fn write(value: &Self::Value, text: &mut String) {
        text.clear();
        if let Some(link) = value {
            text.push_str(link.as_str());
        }
    }

    fn label(language: Language) -> &'static str {
        language.text("Ссылка", "Link")
    }
    fn hint(language: Language) -> &'static str {
        language.text("Ссылка на звонок или событие", "Link to a call or event")
    }
}
