//! Узкий интерфейс инструментов workspace; в обычной сборке feature отключена.

use super::Planner;

pub use crate::model::{
    Availability, CalendarViewMode, CategoryId, DateRange, DisplayTimeZone, Document, Event,
    EventDetails, EventId, EventIdentity, EventListPosition, EventSchedule, EventUid, Frequency,
    InputError, Recurrence, Title, Year,
};
pub use crate::text::{Language, LanguageMode};
pub use crate::vacation::Estimate as VacationEstimate;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
pub const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
pub const HOLIDAYS_VERSION: &str = env!("HOLIDAYS_RU_VERSION");

#[derive(Clone, Copy, Debug)]
pub struct CalendarState {
    pub stride: f32,
    pub offset: f32,
    pub columns: usize,
    pub rect: egui::Rect,
    pub cached_years: usize,
    pub has_events: bool,
}

pub struct SearchState<'a> {
    pub open: bool,
    pub query: &'a str,
    pub selected: usize,
    pub dates: Option<DateRange>,
    pub results: usize,
}

pub struct VacationState<'a> {
    pub open: bool,
    pub estimate: Option<&'a VacationEstimate>,
}

pub struct AppState<'a> {
    pub document: &'a Document,
    pub selection: Option<DateRange>,
    pub calendar: Option<CalendarState>,
    pub search: SearchState<'a>,
    pub event_editor_open: bool,
    pub event_error: Option<InputError>,
    pub about_open: bool,
    pub dirty: bool,
    pub notice: Option<&'a str>,
    pub vacation: VacationState<'a>,
}

#[must_use]
pub fn from_document(document: Document, ctx: &egui::Context) -> Planner {
    Planner::from_document(document, ctx, None)
}

#[must_use]
pub fn from_storage(storage: Option<&dyn eframe::Storage>, ctx: &egui::Context) -> Planner {
    Planner::from_storage(storage, ctx)
}

pub fn render(planner: &mut Planner, ui: &mut egui::Ui) {
    planner.render(ui);
}

#[must_use]
pub fn event_dialog_rect(ctx: &egui::Context) -> Option<egui::Rect> {
    super::dialogs::event_rect(ctx)
}

#[must_use]
pub fn vacation_window_rect(ctx: &egui::Context) -> Option<egui::Rect> {
    ctx.memory(|memory| memory.area_rect(super::dialogs::Dialog::Vacation.id()))
}

impl Planner {
    #[must_use]
    pub fn inspect(&self) -> AppState<'_> {
        AppState {
            document: &self.document,
            selection: self.selection.range(),
            calendar: self.continuous.inspect(),
            search: self.search.inspect(),
            event_editor_open: self.event_draft.is_some(),
            event_error: self.event_draft.as_ref().and_then(super::EventDraft::error),
            about_open: self.about_open,
            dirty: self.persistence.is_dirty(),
            notice: self.notice.as_deref(),
            vacation: VacationState {
                open: self.vacation.open,
                estimate: self
                    .document
                    .vacation
                    .enabled
                    .then(|| self.vacation.estimate())
                    .flatten(),
            },
        }
    }

    /// Подготовка выбранной даты до измерений; пользовательские тесты используют UI.
    ///
    /// # Errors
    /// Возвращает ошибку, если год даты вне поддерживаемого диапазона.
    pub fn select_date(&mut self, date: chrono::NaiveDate) -> Result<(), InputError> {
        self.go_to_date(date)
    }
}
