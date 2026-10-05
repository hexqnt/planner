use crate::model::EventListPosition;

use super::{Planner, sidebar::SidebarTab, widgets};

mod list;

impl Planner {
    pub(super) fn change_event_list_position(&mut self, position: EventListPosition) {
        if self.document.event_list_position != position {
            self.document.event_list_position = position;
            self.persistence.mark_changed();
        }
        if position == EventListPosition::Left {
            self.sidebar_visible = true;
            self.sidebar_tab = SidebarTab::Calendars;
        }
    }

    pub(super) fn embedded_events(&mut self, ui: &mut egui::Ui) {
        self.events_header(ui, EventListPosition::Left);
        self.event_list(ui);
    }

    pub(super) fn events_panel(&mut self, ui: &mut egui::Ui) {
        let language = self.language();
        ui.response().widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::Panel,
                ui.is_enabled(),
                language.text("Список событий", "Event list"),
            )
        });
        self.events_header(ui, EventListPosition::Right);
        ui.separator();
        egui::ScrollArea::vertical()
            .id_salt("events_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| self.event_list(ui));
    }

    fn events_header(&mut self, ui: &mut egui::Ui, position: EventListPosition) {
        let language = self.language();
        ui.horizontal(|ui| {
            widgets::hint(
                ui.strong(language.text("События", "Events")),
                language.text(
                    "Добавьте событие с датами, чтобы раскрасить календарь. Здесь показаны события выбранного интервала или всего года, если даты не выделены.",
                    "Add an event with dates to color the calendar. This list shows events in the selected range, or the whole year when no dates are selected.",
                ),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let (arrow, label, destination) = match position {
                    EventListPosition::Right => (
                        "←",
                        language.text("Перенести события влево", "Move events left"),
                        EventListPosition::Left,
                    ),
                    EventListPosition::Left => (
                        "→",
                        language.text("Перенести события вправо", "Move events right"),
                        EventListPosition::Right,
                    ),
                };
                if widgets::labelled_action(ui.small_button(arrow), label).clicked() {
                    self.change_event_list_position(destination);
                }
            });
        });
        if let Some(range) = self.selection.range() {
            widgets::date_range(ui, range, language);
        } else {
            ui.weak(format!(
                "{} {}",
                language.text("Год", "Year"),
                self.document.year.get()
            ));
        }
        ui.horizontal_wrapped(|ui| {
            if ui
                .button(language.text("+ Событие", "+ Event"))
                .on_hover_text(language.text("Создать событие · N", "Create event · N"))
                .clicked()
            {
                self.new_event();
            }
            if self.selection.range().is_some()
                && ui
                    .button(language.text("Показать весь год", "Show whole year"))
                    .on_hover_text(language.text("Снять выделение · Esc", "Clear selection · Esc"))
                    .clicked()
            {
                self.selection.clear();
            }
        });
    }
}
