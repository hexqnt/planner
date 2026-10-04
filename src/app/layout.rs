use egui::{Color32, Vec2};

use crate::{
    app::ui_config::layout::SIDEBAR_WIDTH,
    calendar::Region,
    model::CalendarViewMode,
    text::{Language, LanguageMode},
};

use super::{
    Planner,
    appearance::{self, theme_button},
    grid, icons,
    ui_config::{layout as config, style},
    widgets,
};

pub(super) mod timezone;
mod year;

impl Planner {
    fn header(&mut self, ui: &mut egui::Ui, inline_timezone: bool) {
        let language = self.language();
        let available = ui.available_rect_before_wrap();
        let row_height = config::HEADER_HEIGHT
            - f32::from(config::HEADER_MARGIN.top + config::HEADER_MARGIN.bottom);
        let rect = egui::Rect::from_min_max(
            available.min,
            egui::pos2(available.right(), available.top() + row_height),
        );
        let layout = egui::Layout::left_to_right(egui::Align::Center);
        let compact_toolbar = rect.width() < 900.0;
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(egui::Rect::from_min_size(
                    rect.min,
                    Vec2::new(
                        config::HEADER_LEFT_WIDTH
                            + if inline_timezone {
                                config::TIMEZONE_WIDTH
                            } else {
                                0.0
                            },
                        rect.height(),
                    ),
                ))
                .layout(layout),
            |ui| {
                if compact_toolbar {
                    ui.spacing_mut().button_padding.x = 4.0;
                }
                ui.spacing_mut().interact_size.y = ui.spacing().button_padding.y.mul_add(
                    2.0,
                    ui.text_style_height(&egui::TextStyle::Button)
                        .max(ui.spacing().icon_width),
                );
                if ui
                    .button("☰")
                    .on_hover_text(language.text("Календари", "Calendars"))
                    .clicked()
                {
                    self.sidebar_visible = !self.sidebar_visible;
                }
                if compact_toolbar {
                    self.search_button(ui);
                }
                self.region_picker(ui);
                if inline_timezone {
                    self.timezone_button(ui);
                }
            },
        );
        if !inline_timezone {
            ui.scope_builder(
                egui::UiBuilder::new()
                    .max_rect(egui::Rect::from_min_max(
                        egui::pos2(available.left(), rect.bottom()),
                        available.max,
                    ))
                    .layout(layout),
                |ui| {
                    ui.weak(language.text("Часовой пояс", "Timezone"));
                    self.timezone_button(ui);
                },
            );
        }
        self.year_picker(ui, rect);
        let right = egui::Rect::from_min_max(
            egui::pos2(rect.right() - config::HEADER_RIGHT_WIDTH, rect.top()),
            rect.max,
        );
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(right)
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
            |ui| {
                self.settings_button(ui);
                if widgets::labelled_action(
                    theme_button(ui, self.document.dark),
                    language.text("Сменить тему", "Switch theme"),
                )
                .clicked()
                {
                    self.document.dark = !self.document.dark;
                    appearance::apply_style(ui.ctx(), self.document.dark);
                    self.persistence.mark_changed();
                }
            },
        );
    }

    fn search_button(&mut self, ui: &mut egui::Ui) {
        if ui
            .add(
                widgets::ActionButton::new(
                    icons::SEARCH,
                    self.language()
                        .text("Поиск событий · Ctrl/⌘ + K", "Search events · Ctrl/⌘ + K"),
                )
                .icon_only(style::TOOLBAR_ICON_SIZE),
            )
            .clicked()
        {
            self.open_search();
        }
    }

    fn region_picker(&mut self, ui: &mut egui::Ui) {
        let language = self.language();
        egui::ComboBox::from_id_salt("region")
            .popup_style(appearance::dropdown_style.into())
            .selected_text(self.document.region.name(language))
            .width(config::REGION_WIDTH)
            .show_ui(ui, |ui| {
                for region in Region::ALL {
                    if ui
                        .selectable_value(&mut self.document.region, region, region.name(language))
                        .changed()
                    {
                        self.persistence.mark_changed();
                    }
                }
            });
    }

    fn settings_button(&mut self, ui: &mut egui::Ui) -> egui::Response {
        let language = self.language();
        ui.scope(|ui| {
            ui.spacing_mut().button_padding = Vec2::new(4.0, 6.0);
            widgets::ActionButton::new(icons::SETTINGS, language.text("Настройки", "Settings"))
                .icon_only(style::TOOLBAR_ICON_SIZE)
                .min_size(config::THEME_BUTTON_SIZE)
                .show_menu(ui, |ui| {
                    ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                        for (mode, label) in [
                            (
                                CalendarViewMode::SingleYear,
                                language.text("Один год", "Single year"),
                            ),
                            (
                                CalendarViewMode::Continuous,
                                language.text("Непрерывный", "Continuous"),
                            ),
                        ] {
                            if ui.radio(self.document.view_mode == mode, label).clicked() {
                                self.change_view_mode(mode);
                                ui.close();
                            }
                        }
                        ui.separator();
                        let icon = egui::Image::new(icons::VACATION)
                            .fit_to_exact_size(style::ACTION_ICON_SIZE)
                            .tint(ui.visuals().text_color());
                        if ui
                            .checkbox(
                                &mut self.document.vacation.enabled,
                                (icon, language.text("Режим отпуска", "Vacation mode")),
                            )
                            .changed()
                        {
                            self.vacation.open = self.document.vacation.enabled;
                            self.persistence.mark_changed();
                            ui.close();
                        }
                        ui.separator();
                        self.language_menu(ui);
                        self.file_menu(ui);
                        ui.separator();
                        if ui
                            .add(widgets::ActionButton::new(
                                icons::INFO,
                                language.text("О программе", "About"),
                            ))
                            .clicked()
                        {
                            self.about_open = true;
                            ui.close();
                        }
                    });
                })
                .response
        })
        .inner
    }

    fn language_menu(&mut self, ui: &mut egui::Ui) {
        let language = self.language();
        let icon = egui::Image::new(icons::LANGUAGE)
            .fit_to_exact_size(style::ACTION_ICON_SIZE)
            .tint(ui.visuals().text_color());
        ui.menu_button((icon, language.text("Язык", "Language")), |ui| {
            let automatic = self.document.language_mode == LanguageMode::Auto;
            if ui
                .selectable_label(automatic, language.text("Авто", "Auto"))
                .on_hover_text(language.text("Язык ОС или браузера", "OS or browser language"))
                .clicked()
            {
                self.document.language_mode = LanguageMode::Auto;
                self.document.resolve_language();
                self.persistence.mark_changed();
                ui.close();
            }
            for (choice, label, flag) in [
                (Language::Russian, "Русский", icons::FLAG_RU),
                (Language::English, "English", icons::FLAG_EN),
            ] {
                let image = egui::Image::new(flag).fit_to_exact_size(Vec2::new(24.0, 16.0));
                if ui
                    .add(
                        egui::Button::image_and_text(image, label)
                            .selected(!automatic && language == choice),
                    )
                    .clicked()
                {
                    self.document.language_mode = LanguageMode::Manual;
                    self.document.language = choice;
                    self.persistence.mark_changed();
                    ui.close();
                }
            }
        });
    }

    fn footer(&mut self, ui: &mut egui::Ui) {
        let language = self.language();
        let response = ui.response();
        response.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::Panel,
                ui.is_enabled(),
                language.text("Действия календаря", "Calendar actions"),
            )
        });
        ui.horizontal_wrapped(|ui| {
            if ui
                .button(language.text("Сегодня", "Today"))
                .on_hover_text(language.text("Перейти к сегодняшнему дню · T", "Go to today · T"))
                .clicked()
            {
                self.go_to_today();
            }
            if let Some(range) = self.selection.range() {
                widgets::date_range(ui, range, language);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(language.text("+ Событие", "+ Event"))
                    .on_hover_text(language.text("Создать событие · N", "Create event · N"))
                    .clicked()
                {
                    self.new_event();
                }
            });
        });
    }
}

impl Planner {
    pub(super) fn render(&mut self, ui: &mut egui::Ui) {
        widgets::apply_pending_action(ui.ctx());
        self.shortcuts(ui.ctx());
        let inline_timezone = ui.available_width()
            - f32::from(config::HEADER_MARGIN.left + config::HEADER_MARGIN.right)
            >= config::TIMEZONE_INLINE_MIN_WIDTH;
        egui::Panel::top("header")
            .exact_size(
                config::HEADER_HEIGHT
                    + if inline_timezone {
                        0.0
                    } else {
                        config::TIMEZONE_ROW_HEIGHT
                    },
            )
            .frame(appearance::panel_frame(ui, config::HEADER_MARGIN))
            .show(ui, |ui| self.header(ui, inline_timezone));
        egui::Panel::bottom("footer")
            .frame(appearance::panel_frame(ui, config::FOOTER_MARGIN))
            .show(ui, |ui| self.footer(ui));
        let search_overlay = self.search.open && ui.available_width() < 1050.0;
        if self.sidebar_visible && !search_overlay {
            egui::Panel::left("sidebar")
                .exact_size(if self.search.open {
                    SIDEBAR_WIDTH
                } else {
                    config::SIDEBAR_WIDTH
                })
                .resizable(false)
                .show_separator_line(false)
                .frame(appearance::panel_frame(ui, config::SIDEBAR_MARGIN))
                .show(ui, |ui| self.sidebar(ui));
        }
        egui::CentralPanel::default()
            .frame(appearance::panel_frame(ui, grid::GRID_MARGIN))
            .show(ui, |ui| {
                let grid_size = Vec2::new(
                    ui.available_width(),
                    (ui.available_height() - config::PERFORMANCE_HEIGHT).max(0.0),
                );
                ui.allocate_ui_with_layout(
                    grid_size,
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| self.year_grid(ui),
                );
                self.performance_indicator(ui);
            });
        self.dialogs(ui.ctx());
        self.file_dialog(ui.ctx());
        if search_overlay {
            self.search_overlay(ui.ctx());
        }
    }

    fn performance_indicator(&self, ui: &mut egui::Ui) {
        let language = self.language();
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let text = self.frame_cpu_seconds.map_or_else(
                || language.text("Задержка кадра: —", "Frame delay: —").to_owned(),
                |seconds| {
                    format!(
                        "{}: {:.1} {}",
                        language.text("Задержка кадра", "Frame delay"),
                        seconds * 1000.0,
                        language.text("мс", "ms"),
                    )
                },
            );
            ui.label(
                egui::RichText::new(text)
                    .size(config::PERFORMANCE_FONT_SIZE)
                    .color(Color32::from_gray(if self.document.dark { config::PERFORMANCE_DARK_GRAY } else { config::PERFORMANCE_LIGHT_GRAY })),
            )
            .on_hover_text(language.text(
                "Время обработки предыдущего кадра на CPU. Меньше — быстрее. Не включает время GPU и полную задержку от ввода до экрана. Обновляется при перерисовке интерфейса.",
                "CPU processing time of the previous frame. Lower is faster. Excludes GPU time and full input-to-display latency. Updates when the interface repaints.",
            ));
        });
    }
}
