use crate::{
    model::{Email, InputError},
    text::Language,
};

use super::{ParsedTextInput, TextDraft, TextKind};

pub(in crate::app) struct EmailListKind;
pub(in crate::app) type EmailListDraft = TextDraft<EmailListKind>;
pub(in crate::app) type EmailListInput<'a> = ParsedTextInput<'a, EmailListKind>;

impl EmailListDraft {
    pub(in crate::app) fn from_emails(emails: &[Email]) -> Self {
        Self::from_value(emails.to_vec())
    }
}

impl TextKind for EmailListKind {
    type Value = Vec<Email>;
    const EMPTY: Self::Value = Vec::new();
    const MULTILINE: bool = true;

    fn parse(text: &str) -> Result<Self::Value, InputError> {
        text.split([',', ';', '\n'])
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::parse)
            .collect()
    }

    fn write(value: &Self::Value, text: &mut String) {
        text.clear();
        for email in value {
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(email.as_str());
        }
    }

    fn label(language: Language) -> &'static str {
        language.text("Участники", "Participants")
    }
    fn hint(language: Language) -> &'static str {
        language.text(
            "Email участников через запятую",
            "Participant emails, comma-separated",
        )
    }
}
