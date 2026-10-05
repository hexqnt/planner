use super::{Planner, appearance, icons, ui_config::sidebar as config, widgets};
use crate::model::EventListPosition;
use tree::TreeView;

mod create;
pub(super) mod rename;
mod tree;

impl Planner {
    /// Возвращает размещение списка событий в текущем кадре; без списка слева нужна правая панель.
    pub(super) fn sidebar(&mut self, ui: &mut egui::Ui) -> EventListPosition {
        self.sidebar_tabs(ui);
        if self.search.open {
            self.search_panel(ui, false);
            return EventListPosition::Right;
        }
        let language = self.language();
        // Размещение фиксируется до обработки переноса, чтобы список не рисовался дважды.
        let event_list_position = self.document.event_list_position;
        let create = egui::ScrollArea::vertical()
            .id_salt("tree_scroll")
            .show(ui, |ui| {
                let tree = ui
                    .scope(|ui| {
                        let right_inset =
                            ui.spacing().scroll.bar_width + ui.spacing().item_spacing.x;
                        ui.set_max_width((ui.available_width() - right_inset).max(0.0));
                        TreeView::new(language, &mut self.tree_draft, &mut self.rename_draft)
                            .show(ui, &mut self.document.groups)
                    })
                    .inner;
                if tree.changed {
                    self.mark_changed();
                }
                if let Some(category) = tree.event {
                    self.new_event_in(category);
                }
                ui.add_space(config::BACKUP_GAP);
                ui.horizontal_wrapped(|ui| {
                    if let Some(json) = self.persistence.recovery_json()
                        && ui
                            .button(language.text("Исходный JSON", "Recovery JSON"))
                            .clicked()
                    {
                        self.backup = Some(json.into());
                    }
                    if self.document.events.is_empty()
                        && ui
                            .button(language.text("Примеры событий", "Example events"))
                            .clicked()
                    {
                        self.document.add_examples();
                        self.mark_changed();
                    }
                });
                if event_list_position == EventListPosition::Left {
                    ui.add_space(config::EVENTS_GAP);
                    self.embedded_events(ui);
                }
                tree.create
            })
            .inner;
        if let Some(target) = create {
            self.create_tree_entry(target);
            ui.ctx().request_repaint();
        }
        event_list_position
    }

    pub(super) fn sidebar_tabs(&mut self, ui: &mut egui::Ui) {
        let language = self.language();
        ui.scope(|ui| {
            ui.spacing_mut().interact_size.y = appearance::control_height(ui);
            ui.horizontal(|ui| {
                for (search, icon, label) in [
                    (
                        false,
                        icons::CALENDAR,
                        language.text("Календари", "Calendars"),
                    ),
                    (true, icons::SEARCH, language.text("Поиск", "Search")),
                ] {
                    let image = egui::Image::new(icon)
                        .fit_to_exact_size(egui::Vec2::splat(ui.spacing().icon_width));
                    let response = ui.add(
                            egui::Button::selectable(self.search.open == search, (image, label))
                                .stroke(egui::Stroke::NONE)
                                .image_tint_follows_text_color(true),
                        );
                    let response = if search {
                        response
                    } else {
                        widgets::hint(response, language.text("Календари — категории событий. Галочки показывают или скрывают их события на датах.", "Calendars are event categories. Checkboxes show or hide their events on dates."))
                    };
                    if response.clicked() {
                        if search {
                            self.open_search();
                        } else {
                            self.search.open = false;
                        }
                    }
                }
            });
        });
        ui.separator();
    }
}
