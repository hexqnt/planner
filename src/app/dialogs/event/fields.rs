use crate::app::widgets::TextEditExt as _;

use crate::app::{
    dialogs::header,
    icons,
    ui_config::{dialog, event as config},
    widgets,
};

use crate::{
    model::{Availability, Document, EventStatus},
    text::Language,
};

use super::{Dialog, EventAction, EventDraft, EventEditorState};

impl EventDraft {
    pub(super) fn header(&mut self, ui: &mut egui::Ui, language: Language) -> bool {
        header(ui, language, |ui| {
            let width =
                (ui.available_width() - dialog::CLOSE_BUTTON_SIZE.x - ui.spacing().item_spacing.x)
                    .max(config::TITLE_MIN_WIDTH);
            let font = egui::FontId::proportional(dialog::FORM_TITLE_FONT_SIZE);
            let title = ui.add(
                egui::TextEdit::singleline(&mut self.title)
                    .id(Dialog::Event.id().with("title"))
                    .font(font.clone())
                    .frame(egui::Frame::NONE)
                    .desired_width(width)
                    .hint_text(
                        egui::RichText::new(language.text("Придумайте название", "Add a title"))
                            .font(font),
                    )
                    .with_context_menu(language),
            );
            title.widget_info(|| {
                egui::WidgetInfo::labeled(
                    egui::WidgetType::TextEdit,
                    title.enabled(),
                    language.text("Придумайте название", "Add a title"),
                )
            });
            if ui.is_visible() && std::mem::take(&mut self.focus_title) {
                title.request_focus();
                ui.ctx().request_repaint();
            }
        })
    }

    pub(super) fn footer(&self, ui: &mut egui::Ui, language: Language) -> EventAction {
        let mut action = EventAction::None;
        ui.horizontal_wrapped(|ui| {
            if widgets::primary_button(ui, language.text("Сохранить", "Save"))
                .on_hover_text(language.text(
                    "Сохранить событие · Ctrl/⌘ + Enter",
                    "Save event · Ctrl/⌘ + Enter",
                ))
                .clicked()
            {
                action = EventAction::Save;
            }
            if ui
                .button(language.text("Отмена", "Cancel"))
                .on_hover_text(
                    language.text("Отменить редактирование · Esc", "Cancel editing · Esc"),
                )
                .clicked()
            {
                action = EventAction::Cancel;
            }
            if self.state.event_id().is_some() {
                let label = if matches!(self.state, EventEditorState::ConfirmingDeletion(_)) {
                    language.text("Удалить навсегда", "Delete permanently")
                } else {
                    language.text("Удалить", "Delete")
                };
                if ui
                    .add(widgets::ActionButton::new(icons::TRASH, label))
                    .clicked()
                {
                    action = EventAction::RequestDeletion;
                }
            }
        });
        action
    }

    pub(super) fn body(&mut self, ui: &mut egui::Ui, document: &Document) -> EventAction {
        let bottom = ui.max_rect().bottom();
        let footer = egui::Panel::bottom("event_footer")
            .frame(egui::Frame::NONE)
            .show_separator_line(false)
            .show(ui, |ui| {
                self.calendar_field(ui, document);
                self.options_fields(ui, document.language);
                if let Some(error) = self.error {
                    widgets::error_label(ui, error.message(document.language));
                }
                self.footer(ui, document.language)
            });
        if footer.response.rect.bottom() > bottom + 0.5 {
            ui.ctx().request_discard("Event footer height changed");
        }
        let fields = egui::ScrollArea::vertical()
            .id_salt("event_fields")
            .auto_shrink([false, false])
            .max_height(ui.available_height())
            .show_viewport(ui, |ui, viewport| {
                self.fields(ui, document, viewport.height())
            });
        match footer.inner {
            EventAction::None => fields.inner,
            action => action,
        }
    }

    fn fields(&mut self, ui: &mut egui::Ui, document: &Document, height: f32) -> EventAction {
        let language = document.language;
        let top = ui.cursor().top();
        self.date_fields(ui, language);
        widgets::FormRow::new(icons::TIMEZONE, language.text("Часовой пояс", "Timezone")).show(
            ui,
            |ui| {
                ui.add_enabled_ui(!self.all_day, |ui| {
                    self.timezone_picker.event_control(
                        ui,
                        language,
                        &mut self.timezone,
                        &document.recent_timezones,
                    );
                });
            },
        );
        let action = self.detail_fields(ui, language);
        widgets::FormRow::new(icons::MAP_PIN, language.text("Место", "Location")).show(ui, |ui| {
            text_field(
                ui,
                &mut self.location,
                "location",
                language.text("Укажите место", "Add a location"),
                language,
            );
        });
        widgets::FormRow::new(icons::VIDEO, language.text("Ссылка", "Link")).show(ui, |ui| {
            ui.add(widgets::LinkInput::new(
                &mut self.link,
                Dialog::Event.id().with("link"),
                language,
            ));
        });
        let description_height =
            (height - (ui.cursor().top() - top) - ui.spacing().item_spacing.y - dialog::FIELD_GAP)
                .max(config::DESCRIPTION_MIN_HEIGHT);
        widgets::FormRow::new(icons::NOTES, language.text("Описание", "Description")).show(
            ui,
            |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.notes)
                        .id(Dialog::Event.id().with("description"))
                        .frame(widgets::form_text_frame(ui))
                        .hint_text(language.text("Добавьте описание", "Add a description"))
                        .desired_width(f32::INFINITY)
                        .desired_rows(1)
                        .min_size(egui::vec2(0.0, description_height))
                        .with_context_menu(language),
                );
            },
        );
        action
    }

    fn date_fields(&mut self, ui: &mut egui::Ui, language: Language) {
        widgets::FormRow::new(icons::CLOCK, language.text("Дата и время", "Date and time")).show(
            ui,
            |ui| {
                ui.horizontal(|ui| {
                    ui.scope(|ui| {
                        let widgets = &mut ui.visuals_mut().widgets;
                        for widget in [
                            &mut widgets.noninteractive,
                            &mut widgets.inactive,
                            &mut widgets.hovered,
                            &mut widgets.active,
                        ] {
                            widget.corner_radius = egui::CornerRadius::ZERO;
                        }
                        ui.checkbox(&mut self.all_day, language.text("Весь день", "All day"));
                    });
                });
                for (key, label, date_label, time_label, date, time) in [
                    (
                        "start",
                        language.text("С", "From"),
                        language.text("Дата начала", "Start date"),
                        language.text("Время начала", "Start time"),
                        &mut self.start,
                        &mut self.start_time,
                    ),
                    (
                        "end",
                        language.text("По", "To"),
                        language.text("Дата окончания", "End date"),
                        language.text("Время окончания", "End time"),
                        &mut self.end,
                        &mut self.end_time,
                    ),
                ] {
                    ui.horizontal_top(|ui| {
                        ui.add_sized(config::DATE_LABEL_SIZE, egui::Label::new(label));
                        ui.add(widgets::DateInput::new(
                            date,
                            Dialog::Event.id().with(("date", label)),
                            date_label,
                            language,
                        ));
                        if !self.all_day {
                            ui.add(widgets::TimeInput::new(
                                time,
                                Dialog::Event.id().with(("time", key)),
                                time_label,
                                language,
                            ));
                        }
                    });
                }
            },
        );
    }

    fn calendar_field(&mut self, ui: &mut egui::Ui, document: &Document) {
        let language = document.language;
        widgets::FormRow::new(icons::CALENDAR, language.text("Календарь", "Calendar")).show(
            ui,
            |ui| {
                ui.add(
                    widgets::CalendarPicker::new(
                        &mut self.category,
                        document,
                        Dialog::Event.id().with("calendar"),
                    )
                    .width(ui.available_width()),
                );
                if document.visible_category(self.category).is_none() {
                    ui.weak(language.text(
                        "Событие скрыто: включите календарь и его группу.",
                        "Event is hidden: enable its calendar and group.",
                    ));
                }
            },
        );
    }

    fn options_fields(&mut self, ui: &mut egui::Ui, language: Language) {
        ui.collapsing(
            language.text("Дополнительно", "More options"),
            |ui| {
                egui::Grid::new("event_options")
                    .num_columns(2)
                    .show(ui, |ui| {
                        ui.label(language.text("Статус", "Status"));
                        widgets::choice(
                            ui,
                            "event_status",
                            &mut self.status,
                            [
                                None,
                                Some(EventStatus::Tentative),
                                Some(EventStatus::Confirmed),
                                Some(EventStatus::Cancelled),
                            ],
                            |status| status_label(status, language),
                        );
                        ui.end_row();
                        ui.label(language.text("Занятость", "Availability"));
                        widgets::choice(
                            ui,
                            "event_availability",
                            &mut self.availability,
                            [Availability::Busy, Availability::Free],
                            |availability| availability_label(availability, language),
                        );
                        ui.end_row();
                    });
            },
        );
    }
}

fn text_field(ui: &mut egui::Ui, value: &mut String, id: &str, hint: &str, language: Language) {
    ui.add(
        widgets::form_input(ui, value)
            .id(Dialog::Event.id().with(id))
            .hint_text(hint)
            .with_context_menu(language),
    );
}

const fn status_label(status: Option<EventStatus>, language: Language) -> &'static str {
    let (ru, en) = match status {
        None => ("Не указан", "Not specified"),
        Some(EventStatus::Tentative) => ("Предварительно", "Tentative"),
        Some(EventStatus::Confirmed) => ("Подтверждено", "Confirmed"),
        Some(EventStatus::Cancelled) => ("Отменено", "Cancelled"),
    };
    language.text(ru, en)
}

const fn availability_label(availability: Availability, language: Language) -> &'static str {
    match availability {
        Availability::Busy => language.text("Занят", "Busy"),
        Availability::Free => language.text("Свободен", "Free"),
    }
}
