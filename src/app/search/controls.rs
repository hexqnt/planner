use crate::app::widgets::TextEditExt as _;

use std::borrow::Cow;

use super::{Dates, Planner};
use crate::app::widgets;
use crate::{
    model::{EventStatus, SearchFilters, StatusScope},
    text::Language,
};

impl Planner {
    pub(super) fn search_controls(&mut self, ui: &mut egui::Ui) -> bool {
        let language = self.language();
        let mut changed = false;
        ui.horizontal_wrapped(|ui| {
            changed |= self.search_date_picker(ui);
            changed |= self.search_calendar_picker(ui);
        });
        if self.search.dates == Dates::Custom {
            ui.horizontal(|ui| {
                ui.label(language.text("С", "From"));
                changed |= ui
                    .add(
                        widgets::text_input(ui, &mut self.search.start)
                            .desired_width(105.0)
                            .hint_text("YYYY-MM-DD")
                            .with_context_menu(language),
                    )
                    .changed();
                ui.label(language.text("По", "To"));
                changed |= ui
                    .add(
                        widgets::text_input(ui, &mut self.search.end)
                            .desired_width(105.0)
                            .hint_text("YYYY-MM-DD")
                            .with_context_menu(language),
                    )
                    .changed();
            });
        }
        ui.collapsing(language.text("Фильтры", "Filters"), |ui| {
            ui.horizontal(|ui| {
                let label = ui.label(language.text("Начало с", "Starts at or after"));
                let response = ui
                    .add(
                        widgets::text_input(ui, &mut self.search.after)
                            .desired_width(85.0)
                            .hint_text("HH:MM")
                            .with_context_menu(language),
                    )
                    .labelled_by(label.id);
                changed |= widgets::hint(response, language.text("В часовом поясе просмотра; события на весь день исключаются при фильтре времени.", "In the display timezone; time filters exclude all-day events.")).changed();
            });
            let before = self.search.filters.status;
            egui::ComboBox::from_id_salt("search_status")
                .selected_text(status_label(before, language))
                .show_ui(ui, |ui| {
                    for status in [
                        StatusScope::Any,
                        StatusScope::Value(None),
                        StatusScope::Value(Some(EventStatus::Tentative)),
                        StatusScope::Value(Some(EventStatus::Confirmed)),
                        StatusScope::Value(Some(EventStatus::Cancelled)),
                    ] {
                        ui.selectable_value(
                            &mut self.search.filters.status,
                            status,
                            status_label(status, language),
                        );
                    }
                });
            changed |= before != self.search.filters.status;
        });
        changed |= self.search_filter_chips(ui);
        changed
    }
    fn search_date_picker(&mut self, ui: &mut egui::Ui) -> bool {
        let language = self.language();
        let before = self.search.dates;
        let mut picked = false;
        let label = if let (Dates::Year, Some(dates)) = (before, self.search.filters.dates) {
            Cow::Owned(format!(
                "{} {}",
                language.text("Год", "Year"),
                dates.start().format("%Y")
            ))
        } else {
            Cow::Borrowed(date_label(before, language))
        };
        egui::ComboBox::from_id_salt("search_dates")
            .width(140.0)
            .truncate()
            .selected_text(label)
            .show_ui(ui, |ui| {
                for dates in [Dates::All, Dates::Year, Dates::Selection, Dates::Custom] {
                    ui.add_enabled_ui(
                        dates != Dates::Selection || self.selection.range().is_some(),
                        |ui| {
                            picked |= ui
                                .selectable_value(
                                    &mut self.search.dates,
                                    dates,
                                    date_label(dates, language),
                                )
                                .clicked();
                        },
                    );
                }
            });
        if picked {
            match self.search.dates {
                Dates::All => self.search.filters.dates = None,
                Dates::Year => self.search.filters.dates = Some(self.document.year.range()),
                Dates::Selection => self.search.filters.dates = self.selection.range(),
                Dates::Custom if before != Dates::Custom => {
                    let dates = self
                        .search
                        .filters
                        .dates
                        .unwrap_or_else(|| self.document.year.range());
                    self.search.start = dates.start().format("%Y-%m-%d").to_string();
                    self.search.end = dates.end().format("%Y-%m-%d").to_string();
                }
                Dates::Custom => {}
            }
            return true;
        }
        false
    }

    fn search_calendar_picker(&mut self, ui: &mut egui::Ui) -> bool {
        ui.add(widgets::CalendarPicker::new(
            &mut self.search.filters.calendars,
            &self.document,
            egui::Id::new("search_calendars"),
        ))
        .changed()
    }

    fn search_filter_chips(&mut self, ui: &mut egui::Ui) -> bool {
        let language = self.language();
        let mut changed = false;
        ui.horizontal_wrapped(|ui| {
            if let Some(dates) = self.search.filters.dates
                && ui
                    .add(widgets::FilterChip::new(dates.to_string(), language))
                    .clicked()
            {
                self.search.dates = Dates::All;
                self.search.filters.dates = None;
                changed = true;
            }
            if self.search.filters.after.is_some()
                && ui
                    .add(widgets::FilterChip::new(
                        format!("{} {}", language.text("С", "From"), self.search.after),
                        language,
                    ))
                    .clicked()
            {
                self.search.after.clear();
                changed = true;
            }
            if self.search.filters.status != StatusScope::Any
                && ui
                    .add(widgets::FilterChip::new(
                        status_label(self.search.filters.status, language),
                        language,
                    ))
                    .clicked()
            {
                self.search.filters.status = StatusScope::Any;
                changed = true;
            }
            if (self.search.filters != SearchFilters::default()
                || self.search.dates == Dates::Custom
                || !self.search.after.is_empty())
                && ui
                    .small_button(language.text("Сбросить фильтры", "Reset filters"))
                    .clicked()
            {
                self.search.filters = SearchFilters::default();
                self.search.dates = Dates::All;
                self.search.after.clear();
                changed = true;
            }
        });
        changed
    }
}

const fn date_label(dates: Dates, language: Language) -> &'static str {
    match dates {
        Dates::All => language.text("Все даты", "All dates"),
        Dates::Year => language.text("Текущий год", "Current year"),
        Dates::Selection => language.text("Выделенный интервал", "Selected range"),
        Dates::Custom => language.text("Свой диапазон", "Custom range"),
    }
}

const fn status_label(status: StatusScope, language: Language) -> &'static str {
    match status {
        StatusScope::Any => language.text("Любой статус", "Any status"),
        StatusScope::Value(None) => language.text("Статус не указан", "Status not specified"),
        StatusScope::Value(Some(EventStatus::Tentative)) => {
            language.text("Предварительно", "Tentative")
        }
        StatusScope::Value(Some(EventStatus::Confirmed)) => {
            language.text("Подтверждено", "Confirmed")
        }
        StatusScope::Value(Some(EventStatus::Cancelled)) => language.text("Отменено", "Cancelled"),
    }
}
