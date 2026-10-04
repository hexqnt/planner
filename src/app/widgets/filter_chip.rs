use egui::{Response, Ui, Widget};

use crate::text::Language;

/// Тип подписи мономорфизируется; готовая строка передаётся без дополнительного копирования.
pub(in crate::app) struct FilterChip<L> {
    label: L,
    language: Language,
}

impl<L: Into<String>> FilterChip<L> {
    pub(in crate::app) const fn new(label: L, language: Language) -> Self {
        Self { label, language }
    }
}

impl<L: Into<String>> Widget for FilterChip<L> {
    fn ui(self, ui: &mut Ui) -> Response {
        let mut label = self.label.into();
        label.push_str(" ×");
        ui.small_button(label)
            .on_hover_text(self.language.text("Убрать фильтр", "Remove filter"))
    }
}
