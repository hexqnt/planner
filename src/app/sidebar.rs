use super::{Planner, appearance, icons, ui_config::sidebar as config, widgets};
use crate::model::EventListPosition;
use tree::TreeView;

mod create;
pub(super) mod rename;
mod tree;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum SidebarTab {
    #[default]
    Calendars,
    Search,
    Chat,
}

impl Planner {
    /// Возвращает размещение списка событий в текущем кадре; без списка слева нужна правая панель.
    pub(super) fn sidebar(&mut self, ui: &mut egui::Ui) -> EventListPosition {
        self.sidebar_tabs(ui);
        let language = self.language();
        match self.sidebar_tab {
            SidebarTab::Search => {
                self.search_panel(ui, false);
                return EventListPosition::Right;
            }
            SidebarTab::Chat => {
                self.chat.show(ui, language, &mut self.shortcuts);
                return EventListPosition::Right;
            }
            SidebarTab::Calendars => {}
        }
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
        let calendars_hint = language.text(
            "Календари — категории событий. Галочки показывают или скрывают их события на датах.",
            "Calendars are event categories. Checkboxes show or hide their events on dates.",
        );
        ui.scope(|ui| {
            ui.spacing_mut().interact_size.y = appearance::control_height(ui);
            ui.spacing_mut().button_padding.x = config::TAB_BUTTON_PADDING;
            ui.spacing_mut().item_spacing.x = config::TAB_GAP;
            ui.horizontal_wrapped(|ui| {
                for (tab, icon, label) in [
                    (
                        SidebarTab::Calendars,
                        icons::CALENDAR,
                        language.text("Календари", "Calendars"),
                    ),
                    (
                        SidebarTab::Search,
                        icons::SEARCH,
                        language.text("Поиск", "Search"),
                    ),
                    (
                        SidebarTab::Chat,
                        icons::MESSAGE_AI,
                        language.text("Чат", "Chat"),
                    ),
                ] {
                    let image = egui::Image::new(icon)
                        .fit_to_exact_size(egui::Vec2::splat(ui.spacing().icon_width));
                    let response = ui.add(
                        egui::Button::selectable(self.sidebar_tab == tab, (image, label))
                            .stroke(egui::Stroke::NONE)
                            .image_tint_follows_text_color(true),
                    );
                    let response = if tab == SidebarTab::Calendars {
                        widgets::hint(response, calendars_hint)
                    } else {
                        response
                    };
                    if response.clicked() {
                        match tab {
                            SidebarTab::Search => self.open_search(),
                            SidebarTab::Calendars | SidebarTab::Chat => self.sidebar_tab = tab,
                        }
                    }
                }
            });
        });
        ui.separator();
    }
}
