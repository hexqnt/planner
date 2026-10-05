//! Состояние поиска не сохраняется в документ и не зависит от навигации календаря.

use chrono::{DateTime, Datelike as _, NaiveDate, NaiveTime, Utc};

use super::{Planner, appearance, dialogs, sidebar::SidebarTab, widgets};
use crate::{
    model::{
        DateRange, DisplayTimeZone, QueryError, SearchFilters, SearchIndex, SearchQuery, Year,
    },
    text::Language,
};

mod controls;
mod results;
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Dates {
    #[default]
    All,
    Year,
    Selection,
    Custom,
}

#[derive(Clone, Copy)]
enum FilterError {
    Dates,
    Time,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct SearchContext {
    today: NaiveDate,
    timezone: DisplayTimeZone,
    language: Language,
}

pub(super) struct Search {
    focus: bool,
    input: String,
    query: Result<SearchQuery, QueryError>,
    filters: SearchFilters,
    index: SearchIndex,
    dates: Dates,
    start: String,
    end: String,
    after: String,
    error: Option<FilterError>,
    selected: usize,
    activate: Option<usize>,
    scroll_selection: bool,
    title_indices: Vec<u32>,
    snippet_indices: Vec<u32>,
    context: Option<SearchContext>,
    reference: Option<DateTime<Utc>>,
}

#[cfg(feature = "testing")]
impl Search {
    pub(super) fn inspect(&self, open: bool) -> super::testing::SearchState<'_> {
        super::testing::SearchState {
            open,
            query: &self.input,
            selected: self.selected,
            dates: self.filters.dates,
            results: self.index.results().len(),
        }
    }
}

impl Default for Search {
    fn default() -> Self {
        Self {
            focus: false,
            input: String::new(),
            query: Ok(SearchQuery::default()),
            filters: SearchFilters::default(),
            index: SearchIndex::default(),
            dates: Dates::All,
            start: String::new(),
            end: String::new(),
            after: String::new(),
            error: None,
            selected: 0,
            activate: None,
            scroll_selection: false,
            title_indices: Vec::new(),
            snippet_indices: Vec::new(),
            context: None,
            reference: None,
        }
    }
}

impl Search {
    pub fn invalidate(&mut self) {
        self.index.invalidate();
        self.reset_selection();
    }

    fn restart(&mut self) {
        self.index.restart();
        self.reset_selection();
    }

    const fn reset_selection(&mut self) {
        self.selected = 0;
        self.activate = None;
        self.reference = None;
        self.scroll_selection = true;
    }

    fn active(&self) -> bool {
        !self.input.trim().is_empty() || self.filters != SearchFilters::default()
    }

    fn parse_filters(&mut self) {
        match self.parsed_filters() {
            Ok(filters) => {
                self.filters = filters;
                self.error = None;
            }
            Err(error) => self.error = Some(error),
        }
    }

    fn parsed_filters(&self) -> Result<SearchFilters, FilterError> {
        let mut filters = self.filters;
        if self.dates == Dates::Custom {
            filters.dates = Some(
                parse_date(&self.start)
                    .zip(parse_date(&self.end))
                    .and_then(|(start, end)| DateRange::new(start, end).ok())
                    .ok_or(FilterError::Dates)?,
            );
        }
        let after = self.after.trim();
        filters.after = if after.is_empty() {
            None
        } else {
            Some(parse_time(after).ok_or(FilterError::Time)?)
        };
        Ok(filters)
    }
}

fn parse_date(text: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(text.trim(), "%Y-%m-%d")
        .ok()
        .filter(|date| Year::try_from(date.year()).is_ok())
}

fn parse_time(text: &str) -> Option<NaiveTime> {
    NaiveTime::parse_from_str(text, "%H:%M")
        .or_else(|_| NaiveTime::parse_from_str(text, "%H:%M:%S"))
        .ok()
}

impl Planner {
    pub(super) fn open_search(&mut self) {
        self.sidebar_visible = true;
        self.sidebar_tab = SidebarTab::Search;
        self.search.focus = true;
        self.search.restart();
    }

    pub(super) fn search_overlay(&mut self, ctx: &egui::Context) {
        let language = self.language();
        let response = egui::Modal::new(egui::Id::new("search_overlay"))
            .frame(dialogs::window_frame(ctx))
            .show(ctx, |ui| {
                let size = ctx.content_rect().size();
                ui.set_width(420.0_f32.min((size.x - 48.0).max(180.0)));
                ui.set_max_height((size.y - 80.0).max(160.0));
                if dialogs::Heading::new(language.text("Поиск событий", "Search events"), language)
                    .show(ui)
                {
                    self.sidebar_tab = SidebarTab::Calendars;
                }
                self.search_panel(ui, true);
            });
        if response.should_close() {
            self.sidebar_tab = SidebarTab::Calendars;
        }
    }

    pub(super) fn search_panel(&mut self, ui: &mut egui::Ui, overlay: bool) {
        ui.scope(|ui| {
            ui.spacing_mut().interact_size.y = appearance::control_height(ui);
            self.search_form(ui, overlay);
        });
    }

    fn search_form(&mut self, ui: &mut egui::Ui, overlay: bool) {
        self.search_keyboard(ui);
        let language = self.language();
        let mut changed = ui
            .add(
                widgets::SearchInput::new(
                    &mut self.search.input,
                    egui::Id::new("event_search_input"),
                    language,
                )
                .request_focus(std::mem::take(&mut self.search.focus)),
            )
            .changed();
        if changed {
            self.search.query = SearchQuery::parse(&self.search.input);
        }
        changed |= self.search_controls(ui);
        if changed {
            self.search.parse_filters();
            self.search.restart();
        }
        let context = SearchContext {
            today: self.document.display_timezone.today(),
            timezone: self.document.display_timezone,
            language,
        };
        if self.search.context != Some(context) {
            self.search.context = Some(context);
            self.search.restart();
        }
        let query = match &self.search.query {
            Ok(query) => query,
            Err(error) => {
                widgets::error_label(ui, error.message(language));
                return;
            }
        };
        if let Some(error) = self.search.error {
            widgets::error_label(
                ui,
                match error {
                    FilterError::Time => {
                        language.text("Время: ЧЧ:ММ или ЧЧ:ММ:СС", "Time: HH:MM or HH:MM:SS")
                    }
                    FilterError::Dates => language.text(
                        "Введите даты ГГГГ-ММ-ДД, начало не позже окончания",
                        "Enter YYYY-MM-DD dates, start must not follow end",
                    ),
                },
            );
            return;
        }
        if !self.search.active() {
            return;
        }
        let reference = *self.search.reference.get_or_insert_with(Utc::now);
        self.search
            .index
            .step(&self.document, query, self.search.filters, reference);
        if self.search.index.is_pending() {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.weak(language.text("Поиск…", "Searching…"));
            });
            ui.ctx().request_repaint();
            return;
        }
        self.search_results(ui, overlay);
    }
}
