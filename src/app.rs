use chrono::{Datelike as _, NaiveDate};

use crate::{
    calendar::Calendar,
    model::{
        CalendarViewMode, CategoryId, DateRange, Document, EventId, EventIndex, InputError, Year,
    },
    text::Language,
};
use dialogs::{EventDraft, TreeDraft};
use persistence::{LoadedDocument, Persistence};
use selection::Selection;
use sidebar::rename::RenameDraft;
use widgets::YearDraft;

use ui_config::style::{BLUE, RED};

mod appearance;
mod dialogs;
mod events;
mod files;
mod grid;
mod icons;
mod layout;
mod persistence;
mod search;
mod selection;
mod shortcuts;
mod sidebar;
mod ui_config;
mod vacation;
mod widgets;

#[cfg(feature = "testing")]
pub mod testing;

#[derive(Clone, Copy, PartialEq, Eq)]
enum TreeTarget {
    Group(usize),
    Category(CategoryId),
}

pub struct Planner {
    document: Document,
    files: files::FileTransfer,
    timezone_picker: layout::timezone::TimeZonePicker,
    calendar: Calendar,
    event_index: EventIndex,
    continuous: grid::ContinuousView,
    about_open: bool,
    selection: Selection,
    event_draft: Option<EventDraft>,
    tree_draft: Option<TreeDraft>,
    rename_draft: Option<RenameDraft>,
    year_draft: Option<YearDraft>,
    backup: Option<String>,
    notice: Option<String>,
    persistence: Persistence,
    sidebar_visible: bool,
    search: search::Search,
    shortcuts: shortcuts::Shortcuts,
    frame_cpu_seconds: Option<f32>,
    vacation: vacation::Vacation,
}

impl Planner {
    #[must_use]
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        let loaded = Persistence::load_native();
        #[cfg(target_arch = "wasm32")]
        let loaded = Persistence::load(cc.storage);
        Self::from_loaded(loaded, &cc.egui_ctx)
    }

    #[cfg(any(test, feature = "testing"))]
    fn from_storage(storage: Option<&dyn eframe::Storage>, ctx: &egui::Context) -> Self {
        Self::from_loaded(Persistence::load(storage), ctx)
    }

    fn from_loaded(loaded: LoadedDocument, ctx: &egui::Context) -> Self {
        let LoadedDocument {
            document,
            persistence,
            error,
        } = loaded;
        let notice = error.map(|error| {
            let recovery_hint = if persistence.recovery_json().is_some() {
                document.language.text(
                    "Исходные данные доступны по кнопке «Исходный JSON».",
                    "Original data is available via Recovery JSON.",
                )
            } else {
                ""
            };
            format!(
                "{}: {error}. {recovery_hint}",
                document.language.text(
                    "Не удалось загрузить сохранённый календарь",
                    "Unable to load saved calendar"
                ),
            )
        });
        let mut planner = Self::from_document(document, ctx, notice);
        planner.persistence = persistence;
        planner
    }

    fn from_document(mut document: Document, ctx: &egui::Context, notice: Option<String>) -> Self {
        document.resolve_language();
        egui_extras::install_image_loaders(ctx);
        appearance::apply_fonts(ctx);
        appearance::apply_style(ctx, document.dark);
        Self {
            calendar: Calendar::new(document.year, document.region),
            vacation: vacation::Vacation::new(document.vacation),
            document,
            files: files::FileTransfer::default(),
            timezone_picker: layout::timezone::TimeZonePicker::default(),
            event_index: EventIndex::default(),
            continuous: grid::ContinuousView::default(),
            about_open: false,
            selection: Selection::default(),
            event_draft: None,
            tree_draft: None,
            rename_draft: None,
            year_draft: None,
            backup: None,
            notice,
            persistence: Persistence::default(),
            sidebar_visible: true,
            search: search::Search::default(),
            shortcuts: shortcuts::Shortcuts::default(),
            frame_cpu_seconds: None,
        }
    }

    const fn language(&self) -> Language {
        self.document.language
    }

    fn mark_changed(&mut self) {
        self.persistence.mark_changed();
        self.event_index.invalidate();
        self.continuous.invalidate();
        self.search.invalidate();
    }

    const fn change_year(&mut self, year: Year) {
        self.document.year = year;
        self.selection.clear();
        self.persistence.mark_changed();
        self.event_index.invalidate();
        self.continuous.jump_to(year, None);
    }

    fn change_view_mode(&mut self, mode: CalendarViewMode) {
        if self.document.view_mode != mode {
            self.document.view_mode = mode;
            self.continuous.jump_to(self.document.year, None);
            self.persistence.mark_changed();
        }
    }

    fn go_to_date(&mut self, date: NaiveDate) -> Result<(), InputError> {
        let year = Year::try_from(date.year())?;
        self.change_year(year);
        self.selection.click(date, false);
        self.continuous.jump_to(year, Some(date));
        Ok(())
    }

    fn go_to_today(&mut self) {
        if let Err(error) = self.go_to_date(self.document.display_timezone.today()) {
            self.notice = Some(error.message(self.language()).into());
        }
    }

    fn new_event(&mut self) {
        let Some(category) = self
            .document
            .last_event_category
            .and_then(|id| self.document.category(id))
            .or_else(|| self.document.visible_categories().next())
            .or_else(|| self.document.categories().next())
        else {
            self.notice = Some(
                self.language()
                    .text(
                        "Сначала добавьте календарь в группу.",
                        "Add a calendar to a group first.",
                    )
                    .into(),
            );
            return;
        };
        self.open_new_event(category.id);
    }

    fn new_event_in(&mut self, category: CategoryId) {
        self.remember_event_category(category);
        self.open_new_event(category);
    }

    fn remember_event_category(&mut self, category: CategoryId) {
        if self.document.last_event_category != Some(category) {
            self.document.last_event_category = Some(category);
            self.persistence.mark_changed();
        }
    }

    fn open_new_event(&mut self, category: CategoryId) {
        let today = self.document.display_timezone.today();
        let date = if today.year() == self.document.year.get() {
            today
        } else {
            self.document.year.range().start()
        };
        let range = self
            .selection
            .range()
            .unwrap_or_else(|| DateRange::between(date, date));
        let mut draft = EventDraft::new(range, category);
        draft.set_default_timezone(self.document.display_timezone);
        self.event_draft = Some(draft);
    }

    fn edit_event(&mut self, id: EventId) {
        if let Some(event) = self.document.events.iter().find(|event| event.id == id) {
            let mut draft = EventDraft::from_event(event);
            if event.schedule.times().is_none() {
                draft.set_default_timezone(self.document.display_timezone);
            }
            self.event_draft = Some(draft);
        }
    }

    fn replace_document(&mut self, mut document: Document, ctx: &egui::Context) {
        document.resolve_language();
        self.document = document;
        self.vacation = vacation::Vacation::new(self.document.vacation);
        self.continuous = grid::ContinuousView::default();
        self.selection.clear();
        self.event_draft = None;
        self.tree_draft = None;
        self.rename_draft = None;
        self.year_draft = None;
        self.backup = None;
        self.files = files::FileTransfer::default();
        self.timezone_picker.reset_search();
        self.search = search::Search::default();
        appearance::apply_style(ctx, self.document.dark);
        self.mark_changed();
        ctx.request_repaint();
    }
}

impl eframe::App for Planner {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.frame_cpu_seconds = frame.info().cpu_usage;
        self.render(ui);
        if self.persistence.is_dirty()
            && let Some(storage) = frame.storage_mut()
        {
            self.save(storage);
            storage.flush();
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        if let Err(error) = self.persistence.save(&self.document, storage) {
            self.notice = Some(format!("Unable to save calendar: {error}"));
        }
    }
}

#[cfg(test)]
mod tests;
