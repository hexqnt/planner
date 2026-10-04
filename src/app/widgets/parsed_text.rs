use egui::{Id, Response, Ui, Widget};

use super::TextEditExt as _;
use crate::{model::InputError, text::Language};

mod email_list;
mod link;

pub(in crate::app) use email_list::{EmailListDraft, EmailListInput};
pub(in crate::app) use link::{LinkDraft, LinkInput};

/// Политика поля мономорфизируется вместе с разбором, форматированием и представлением.
pub(in crate::app) trait TextKind {
    type Value;
    /// Готовое значение пустого поля, которое не требует разбора.
    const EMPTY: Self::Value;
    const MULTILINE: bool;
    fn parse(text: &str) -> Result<Self::Value, InputError>;
    fn write(value: &Self::Value, text: &mut String);
    fn label(language: Language) -> &'static str;
    fn hint(language: Language) -> &'static str;
}

/// Разобранное значение обновляется только при изменении текста; незавершённый ввод остаётся в черновике.
pub(in crate::app) struct TextDraft<K: TextKind> {
    text: String,
    value: Result<K::Value, InputError>,
    committed: bool,
}

impl<K: TextKind> Default for TextDraft<K> {
    fn default() -> Self {
        Self {
            text: String::new(),
            value: Ok(K::EMPTY),
            committed: false,
        }
    }
}

impl<K: TextKind> From<String> for TextDraft<K> {
    fn from(text: String) -> Self {
        let value = K::parse(&text);
        Self {
            text,
            value,
            committed: false,
        }
    }
}

impl<K: TextKind> From<&str> for TextDraft<K> {
    fn from(text: &str) -> Self {
        text.to_owned().into()
    }
}

impl<K: TextKind> TextDraft<K> {
    fn from_value(value: K::Value) -> Self {
        let mut text = String::new();
        K::write(&value, &mut text);
        Self {
            text,
            value: Ok(value),
            committed: false,
        }
    }

    pub(in crate::app) fn value(&self) -> Result<&K::Value, InputError> {
        self.value.as_ref().map_err(|error| *error)
    }

    /// Переносит готовое значение после разбора остальных полей; при ошибке сохраняет черновик.
    pub(in crate::app) fn take(&mut self) -> Result<K::Value, InputError> {
        self.value()?;
        std::mem::take(self).value
    }

    #[cfg(test)]
    pub(in crate::app) fn as_str(&self) -> &str {
        &self.text
    }

    #[cfg(test)]
    pub(in crate::app) fn clear(&mut self) {
        *self = Self::default();
    }

    fn reparse(&mut self) {
        self.value = K::parse(&self.text);
        self.committed = false;
    }

    fn finish_edit(&mut self) {
        if self.committed {
            return;
        }
        self.committed = true;
        if let Ok(value) = &self.value {
            K::write(value, &mut self.text);
        }
    }
}

pub(in crate::app) struct ParsedTextInput<'a, K: TextKind> {
    draft: &'a mut TextDraft<K>,
    id: Id,
    language: Language,
}

impl<'a, K: TextKind> ParsedTextInput<'a, K> {
    pub(in crate::app) const fn new(
        draft: &'a mut TextDraft<K>,
        id: Id,
        language: Language,
    ) -> Self {
        Self {
            draft,
            id,
            language,
        }
    }
}

impl<K: TextKind> Widget for ParsedTextInput<'_, K> {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.vertical(|ui| {
            let edit = if K::MULTILINE {
                egui::TextEdit::multiline(&mut self.draft.text).desired_rows(1)
            } else {
                egui::TextEdit::singleline(&mut self.draft.text)
            };
            let response = ui.add(
                edit.id(self.id)
                    .frame(super::form_text_frame(ui))
                    .desired_width(f32::INFINITY)
                    .hint_text(K::hint(self.language))
                    .with_context_menu(self.language),
            );
            response.widget_info(|| {
                egui::WidgetInfo::labeled(
                    egui::WidgetType::TextEdit,
                    response.enabled(),
                    K::label(self.language),
                )
            });
            if response.changed() {
                self.draft.reparse();
            }
            if response.lost_focus() {
                self.draft.finish_edit();
            }
            if self.draft.committed
                && let Err(error) = self.draft.value()
            {
                super::error_label(ui, error.message(self.language));
            }
            response
        })
        .inner
    }
}

#[cfg(test)]
mod tests;
