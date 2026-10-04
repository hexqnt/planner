use std::num::NonZeroU32;

use egui::{Id, Response, Ui, Widget};

use super::TextEditExt as _;
use crate::{text::Language, vacation::Amount};

/// Текст допускает незавершённый ввод; корректная сумма разбирается только при изменении текста.
pub(in crate::app) struct MoneyDraft {
    text: String,
    amount: Option<Amount>,
    language: Language,
    committed: bool,
}

impl MoneyDraft {
    pub(in crate::app) fn new(amount: Amount) -> Self {
        Self {
            text: format_amount(amount.cents(), Language::English),
            amount: Some(amount),
            language: Language::English,
            committed: false,
        }
    }

    pub(in crate::app) fn from_cents(cents: i64) -> Self {
        let text = format_amount(cents, Language::English);
        let amount = u32::try_from(cents)
            .ok()
            .and_then(NonZeroU32::new)
            .map(Amount::from_cents);
        Self {
            text,
            amount,
            language: Language::English,
            committed: false,
        }
    }

    pub(in crate::app) fn finish_edit(&mut self, language: Language) -> bool {
        self.committed = true;
        self.normalize(language)
    }

    fn normalize(&mut self, language: Language) -> bool {
        self.language = language;
        if let Some(amount) = self.amount {
            let normalized = format_amount(amount.cents(), language);
            if self.text != normalized {
                self.text = normalized;
                return true;
            }
        }
        false
    }

    pub(in crate::app) const fn value(&self) -> Option<Amount> {
        self.amount
    }
}

fn format_amount(cents: i64, language: Language) -> String {
    format!(
        "{}{}{:02}",
        cents / 100,
        language.text(",", "."),
        cents % 100
    )
}

pub(in crate::app) struct MoneyInput<'a> {
    draft: &'a mut MoneyDraft,
    id: Id,
    label: &'a str,
    language: Language,
}

impl<'a> MoneyInput<'a> {
    pub(in crate::app) const fn new(
        draft: &'a mut MoneyDraft,
        id: Id,
        label: &'a str,
        language: Language,
    ) -> Self {
        Self {
            draft,
            id,
            label,
            language,
        }
    }
}

impl Widget for MoneyInput<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let draft = self.draft;
        if draft.language != self.language && !ui.memory(|memory| memory.has_focus(self.id)) {
            draft.normalize(self.language);
        }
        ui.vertical(|ui| {
            let mut response = ui.add(
                super::form_input(ui, &mut draft.text)
                    .id(self.id)
                    .with_context_menu(self.language),
            );
            response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, response.enabled(), self.label)
            });
            if response.changed() {
                draft.amount = draft.text.parse().ok();
                draft.committed = false;
            }
            if response.lost_focus() && draft.finish_edit(self.language) {
                response.mark_changed();
            }
            if draft.committed && draft.amount.is_none() {
                let error = self.language.text(
                    "Введите положительную сумму до 42 949 672,95 ₽, не более двух знаков после запятой.",
                    "Enter a positive amount up to RUB 42,949,672.95 with at most two decimal places.",
                );
                super::error_label(ui, error);
            }
            response
        }).inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formatting_preserves_cents_and_uses_the_language_separator() {
        let amount: Amount = "80 000,05".parse().unwrap();
        assert_eq!(format_amount(amount.cents(), Language::Russian), "80000,05");
        assert_eq!(format_amount(amount.cents(), Language::English), "80000.05");
        let draft = MoneyDraft::from_cents(0);
        assert!(draft.value().is_none());
        assert!(MoneyDraft::from_cents(-1).value().is_none());
        assert!(
            MoneyDraft::from_cents(i64::from(u32::MAX) + 1)
                .value()
                .is_none()
        );
        assert_eq!(MoneyDraft::from_cents(amount.cents()).value(), Some(amount));
        assert_eq!(MoneyDraft::new(amount).value(), Some(amount));
    }
}
