use egui::{Id, Response, Ui, Widget};

use super::TextEditExt as _;
use crate::text::Language;

pub(in crate::app) struct SearchInput<'a> {
    text: &'a mut String,
    id: Id,
    language: Language,
    focus: bool,
}

impl<'a> SearchInput<'a> {
    pub(in crate::app) const fn new(text: &'a mut String, id: Id, language: Language) -> Self {
        Self {
            text,
            id,
            language,
            focus: false,
        }
    }

    pub(in crate::app) const fn request_focus(mut self, focus: bool) -> Self {
        self.focus = focus;
        self
    }
}

impl Widget for SearchInput<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let label = self.language.text("Поиск событий…", "Search events…");
        let response = ui
            .horizontal(|ui| {
                let height = ui.spacing().interact_size.y;
                let width = (ui.available_width() - height - ui.spacing().item_spacing.x).max(40.0);
                let mut response = ui.add(
                    super::text_input(ui, self.text)
                        .id(self.id)
                        .desired_width(width)
                        .hint_text(label)
                        .with_context_menu(self.language),
                );
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, response.enabled(), label)
                });
                if self.focus {
                    response.request_focus();
                }
                let clear = ui.add_sized([height, height], egui::Button::new("×"));
                if super::labelled_action(
                    clear,
                    self.language.text("Очистить запрос", "Clear query"),
                )
                .clicked()
                {
                    if !self.text.is_empty() {
                        self.text.clear();
                        response.mark_changed();
                    }
                    response.request_focus();
                }
                response
            })
            .inner;
        super::hint(response, self.language.text(
            "Название, описание, место, email или ссылка. Можно добавить дату или время начала: 2026-10-12 10:00.",
            "Title, description, location, email or link. Add a date or start time: 2026-10-12 10:00.",
        ))
    }
}
