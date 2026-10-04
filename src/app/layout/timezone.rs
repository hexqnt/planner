use crate::app::widgets::TextEditExt as _;

use chrono_tz::Tz;

use crate::{
    model::{DisplayTimeZone, EventTimeZone, RecentTimeZones},
    text::Language,
};

use super::{Planner, config};

pub(in crate::app) struct TimeZonePicker {
    search: ZoneSearch,
    system_name: Option<String>,
    request_focus: bool,
}

impl Default for TimeZonePicker {
    fn default() -> Self {
        Self {
            search: ZoneSearch::default(),
            system_name: iana_time_zone::get_timezone().ok(),
            request_focus: true,
        }
    }
}

/// Буфер результатов переиспользуется: фильтрация нужна только при изменении запроса.
struct ZoneSearch {
    query: String,
    matches: Vec<Tz>,
    reset_scroll: bool,
}

impl Default for ZoneSearch {
    fn default() -> Self {
        Self {
            query: String::new(),
            matches: chrono_tz::TZ_VARIANTS.to_vec(),
            reset_scroll: true,
        }
    }
}

impl ZoneSearch {
    fn refresh(&mut self) {
        self.matches.clear();
        self.matches.extend(
            chrono_tz::TZ_VARIANTS
                .iter()
                .copied()
                .filter(|zone| matches_search(zone.name(), &self.query)),
        );
        self.reset_scroll = true;
    }
}

impl TimeZonePicker {
    pub(in crate::app) fn reset_search(&mut self) {
        self.search.query.clear();
        self.search.refresh();
        self.request_focus = true;
    }

    fn search_field(&mut self, ui: &mut egui::Ui, language: Language) {
        let response = ui.add(
            egui::TextEdit::singleline(&mut self.search.query)
                .id_salt("timezone_search")
                .hint_text(language.text("Поиск города или зоны", "Search city or timezone"))
                .desired_width(f32::INFINITY)
                .with_context_menu(language),
        );
        response.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::TextEdit,
                response.enabled(),
                language.text("Поиск города или зоны", "Search city or timezone"),
            )
        });
        if std::mem::take(&mut self.request_focus) {
            response.request_focus();
        }
        if response.changed() {
            self.search.refresh();
        }
    }

    fn zone_list(
        &mut self,
        ui: &mut egui::Ui,
        language: Language,
        current: Option<Tz>,
    ) -> Option<Tz> {
        let mut selected = None;
        if self.search.matches.is_empty() {
            ui.weak(language.text("Зоны не найдены", "No matching timezones"));
        }
        let scroll = egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .max_height(ui.available_height().max(0.0));
        let scroll = if std::mem::take(&mut self.search.reset_scroll) {
            scroll.vertical_scroll_offset(0.0)
        } else {
            scroll
        };
        scroll.show_rows(
            ui,
            ui.spacing().interact_size.y,
            self.search.matches.len(),
            |ui, rows| {
                for &zone in &self.search.matches[rows] {
                    if ui
                        .selectable_label(current == Some(zone), zone.name())
                        .clicked()
                    {
                        selected = Some(zone);
                    }
                }
            },
        );
        selected
    }

    pub(in crate::app) fn event_control(
        &mut self,
        ui: &mut egui::Ui,
        language: Language,
        current: &mut EventTimeZone,
        recent: &RecentTimeZones,
    ) {
        let label = match *current {
            EventTimeZone::Floating => language.text("Местное время", "Local time"),
            zone => zone.name(),
        };
        let response = egui::ComboBox::from_id_salt("event_timezone")
            .popup_style(crate::app::appearance::dropdown_style.into())
            .selected_text(label)
            .width(ui.available_width())
            .height(320.0)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show_ui(ui, |ui| {
                self.search_field(ui, language);
                ui.separator();
                if ui
                    .selectable_value(
                        current,
                        EventTimeZone::Floating,
                        language.text("Местное время", "Local time"),
                    )
                    .clicked()
                {
                    ui.close();
                }
                if ui
                    .selectable_value(current, EventTimeZone::Utc, "UTC")
                    .clicked()
                {
                    ui.close();
                }
                if self.search.query.trim().is_empty() && recent.iter().next().is_some() {
                    ui.separator();
                    ui.weak(language.text("Недавние", "Recent"));
                    for zone in recent.iter() {
                        if ui
                            .selectable_value(current, EventTimeZone::Named(zone), zone.name())
                            .clicked()
                        {
                            ui.close();
                        }
                    }
                }
                ui.separator();
                if let Some(zone) = self.zone_list(
                    ui,
                    language,
                    match *current {
                        EventTimeZone::Named(zone) => Some(zone),
                        EventTimeZone::Floating | EventTimeZone::Utc => None,
                    },
                ) {
                    *current = EventTimeZone::Named(zone);
                    ui.close();
                }
            })
            .response;
        if response.clicked() {
            self.reset_search();
        }
    }

    fn choices(
        &mut self,
        ui: &mut egui::Ui,
        language: Language,
        current: DisplayTimeZone,
        recent: &RecentTimeZones,
    ) -> Option<DisplayTimeZone> {
        let mut selected = None;
        ui.strong(language.text("Часовой пояс отображения", "Display timezone"));
        self.search_field(ui, language);
        ui.separator();
        if ui
            .selectable_label(
                current == DisplayTimeZone::System,
                language.text("Системная", "System"),
            )
            .clicked()
        {
            selected = Some(DisplayTimeZone::System);
        }
        if let Some(name) = &self.system_name {
            ui.weak(name);
        }
        if ui
            .selectable_label(current == DisplayTimeZone::Utc, "UTC")
            .clicked()
        {
            selected = Some(DisplayTimeZone::Utc);
        }
        if self.search.query.trim().is_empty() && recent.iter().next().is_some() {
            ui.separator();
            ui.weak(language.text("Недавние", "Recent"));
            for zone in recent.iter() {
                if zone_choice(ui, current, zone) {
                    selected = Some(DisplayTimeZone::Named(zone));
                }
            }
        }
        ui.separator();
        if let Some(zone) = self.zone_list(
            ui,
            language,
            match current {
                DisplayTimeZone::Named(zone) => Some(zone),
                DisplayTimeZone::System | DisplayTimeZone::Utc => None,
            },
        ) {
            selected = Some(DisplayTimeZone::Named(zone));
        }
        if selected.is_some() {
            ui.close();
        }
        selected
    }
}

fn zone_choice(ui: &mut egui::Ui, current: DisplayTimeZone, zone: Tz) -> bool {
    ui.push_id(zone, |ui| {
        ui.selectable_label(current == DisplayTimeZone::Named(zone), zone.name())
            .clicked()
    })
    .inner
}

fn matches_search(name: &str, query: &str) -> bool {
    query.split_whitespace().all(|term| {
        name.as_bytes()
            .windows(term.len())
            .any(|part| part.eq_ignore_ascii_case(term.as_bytes()))
    })
}

fn paint_dropdown_arrow(ui: &egui::Ui, response: &egui::Response) {
    let size = ui.spacing().icon_width;
    let center = egui::pos2(
        size.mul_add(-0.5, response.rect.right() - ui.spacing().button_padding.x),
        response.rect.center().y,
    );
    let rect = egui::Rect::from_center_size(center, egui::vec2(size * 0.7, size * 0.45));
    ui.painter().add(egui::Shape::convex_polygon(
        vec![rect.left_top(), rect.right_top(), rect.center_bottom()],
        ui.style().interact(response).fg_stroke.color,
        egui::Stroke::NONE,
    ));
}

impl Planner {
    pub(super) fn timezone_button(&mut self, ui: &mut egui::Ui) -> egui::Response {
        let language = self.language();
        let current = self.document.display_timezone;
        let label = match current {
            DisplayTimeZone::System => language.text("Системная", "System"),
            DisplayTimeZone::Utc => "UTC",
            DisplayTimeZone::Named(zone) => zone.name(),
        };
        let button = egui::Button::new(label)
            .right_text(egui::Atom::custom(
                egui::Id::new("timezone_arrow"),
                egui::Vec2::splat(ui.spacing().icon_width),
            ))
            .truncate()
            .min_size(egui::vec2(config::TIMEZONE_WIDTH, 0.0));
        let mut selected = None;
        let response = ui
            .scope_builder(
                egui::UiBuilder::new().id(egui::Id::new("display_timezone_control")),
                |ui| {
                    egui::containers::menu::MenuButton::from_button(button)
                        .config(
                            egui::containers::menu::MenuConfig::new()
                                .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside),
                        )
                        .ui(ui, |ui| {
                            let maximum = (ui.ctx().content_rect().size() - egui::vec2(32.0, 32.0))
                                .max(egui::Vec2::ZERO)
                                .min(config::TIMEZONE_MENU_MAX_SIZE);
                            selected = egui::Resize::default()
                                .id(egui::Id::new("display_timezone_menu_size"))
                                .default_size(config::TIMEZONE_MENU_DEFAULT_SIZE)
                                .min_size(config::TIMEZONE_MENU_MIN_SIZE.min(maximum))
                                .max_size(maximum)
                                .resizable(true)
                                .with_stroke(false)
                                .show(ui, |ui| {
                                    let height = ui.available_height();
                                    // Общая прокрутка удерживает рамку в лимите, даже если недавние зоны не помещаются.
                                    egui::ScrollArea::vertical()
                                        .id_salt("timezone_menu_contents")
                                        .max_height(height)
                                        .auto_shrink([false, false])
                                        .show(ui, |ui| {
                                            ui.set_max_height(height);
                                            self.timezone_picker.choices(
                                                ui,
                                                language,
                                                current,
                                                &self.document.recent_timezones,
                                            )
                                        })
                                        .inner
                                });
                        })
                        .0
                },
            )
            .inner;
        paint_dropdown_arrow(ui, &response);
        if response.clicked() {
            self.timezone_picker.reset_search();
        }
        if let Some(zone) = selected {
            self.change_display_timezone(zone);
        }
        response.on_hover_ui(|ui| {
            ui.strong(language.text("Часовой пояс отображения", "Display timezone"));
            ui.label(label);
            if current == DisplayTimeZone::System
                && let Some(name) = &self.timezone_picker.system_name
            {
                ui.weak(name);
            }
        })
    }

    pub(in crate::app) fn change_display_timezone(&mut self, zone: DisplayTimeZone) {
        if self.document.display_timezone == zone {
            return;
        }
        self.document.display_timezone = zone;
        if let DisplayTimeZone::Named(zone) = zone {
            self.document.recent_timezones.remember(zone);
        }
        self.mark_changed();
    }
}

#[cfg(test)]
mod tests;
