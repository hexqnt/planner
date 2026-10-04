use crate::model::{CategoryId, Document, Event};

use super::{Planner, icons, widgets};

#[cfg(target_arch = "wasm32")]
use super::dialogs::{Dialog, Heading};

#[derive(Clone, Copy)]
enum FileAction {
    Import,
    Export,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy)]
enum Operation {
    Import(CategoryId),
    Export,
}

#[derive(Default)]
pub(super) struct FileTransfer {
    #[cfg(not(target_arch = "wasm32"))]
    pending: Option<PendingFile>,
    #[cfg(target_arch = "wasm32")]
    import_category: Option<CategoryId>,
    #[cfg(target_arch = "wasm32")]
    browser: browser::BrowserFiles,
}

#[cfg(not(target_arch = "wasm32"))]
struct PendingFile {
    dialog: egui_file_dialog::FileDialog,
    operation: Operation,
}

impl FileTransfer {
    pub(super) const fn dialog_open(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.pending.is_some()
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.import_category.is_some()
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl FileTransfer {
    pub(super) fn cancel_import(&mut self) {
        self.import_category = None;
        self.browser.reset();
    }
}

fn import_options(ui: &mut egui::Ui, document: &Document, category: &mut CategoryId) {
    let language = document.language;
    ui.label(language.text("Целевой календарь", "Destination calendar"));
    let response = ui.add(widgets::CalendarPicker::new(
        category,
        document,
        egui::Id::new("import_calendar"),
    ));
    widgets::hint(response, language.text("Совпадающие UID обновляются. Новые события добавляются в выбранный календарь. Участники, вложения и напоминания пока не переносятся.", "Matching UIDs are updated. New events go into the selected calendar. Participants, attachments and reminders are not transferred yet."));
}

#[cfg(not(target_arch = "wasm32"))]
fn native_dialog(title: &str) -> egui_file_dialog::FileDialog {
    const FILTER: &str = "iCalendar (.ics)";
    egui_file_dialog::FileDialog::new()
        .title(title)
        .add_file_filter(
            FILTER,
            egui_file_dialog::Filter::new(|path: &std::path::Path| {
                path.extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("ics"))
            }),
        )
        .default_file_filter(FILTER)
        .default_file_name("planner.ics")
        .add_save_extension(FILTER, "ics")
        .default_save_extension(FILTER)
}

impl Planner {
    pub(super) fn file_menu(&mut self, ui: &mut egui::Ui) {
        let language = self.language();
        ui.menu_button(
            language.text("Импорт/экспорт", "Import/export"),
            |ui| {
                for (icon, label, action) in [
                    (
                        icons::FILE_IMPORT,
                        language.text("Импорт .ics", "Import .ics"),
                        FileAction::Import,
                    ),
                    (
                        icons::FILE_EXPORT,
                        language.text("Экспорт .ics", "Export .ics"),
                        FileAction::Export,
                    ),
                ] {
                    if ui.add(widgets::ActionButton::new(icon, label)).clicked() {
                        match action {
                            FileAction::Import => self.import_file(),
                            FileAction::Export => self.export_file(),
                        }
                        ui.close();
                    }
                }
                ui.separator();
                if ui
                    .button(language.text("Копия JSON", "JSON backup"))
                    .clicked()
                {
                    match serde_json::to_string_pretty(&self.document) {
                        Ok(json) => self.backup = Some(json),
                        Err(error) => self.notice = Some(error.to_string()),
                    }
                    ui.close();
                }
            },
        );
    }

    pub(super) fn import_file(&mut self) {
        let Some(category) = self
            .document
            .visible_categories()
            .next()
            .or_else(|| self.document.categories().next())
            .map(|category| category.id)
        else {
            self.notice = Some(
                self.language()
                    .text("Сначала добавьте календарь.", "Add a calendar first.")
                    .into(),
            );
            return;
        };
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut dialog =
                native_dialog(self.language().text("Импорт iCalendar", "Import iCalendar"));
            dialog.pick_file();
            self.files.pending = Some(PendingFile {
                dialog,
                operation: Operation::Import(category),
            });
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.files.browser.reset();
            self.files.import_category = Some(category);
        }
    }

    pub(super) fn export_file(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut dialog = native_dialog(
                self.language()
                    .text("Экспорт iCalendar", "Export iCalendar"),
            );
            dialog.save_file();
            self.files.pending = Some(PendingFile {
                dialog,
                operation: Operation::Export,
            });
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.files.browser.reset();
            self.files.import_category = None;
            if let Err(error) = crate::ical::export(&self.document)
                .map_err(|error| error.to_string())
                .and_then(|text| browser::download(&text))
            {
                self.notice = Some(error);
            }
        }
    }

    fn import_calendar(&mut self, text: &str, category: CategoryId) -> Result<(), String> {
        if self.document.category(category).is_none() {
            return Err("Destination calendar no longer exists".into());
        }
        let events = crate::ical::import(text, category).map_err(|error| error.to_string())?;
        self.merge_events(events);
        Ok(())
    }

    fn merge_events(&mut self, events: Vec<Event>) {
        let count = events.len();
        self.document.merge_events(events);
        self.event_draft = None;
        self.mark_changed();
        self.notice = Some(format!(
            "{}: {count}",
            self.language()
                .text("Событий импортировано", "Events imported")
        ));
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn file_dialog(&mut self, ctx: &egui::Context) {
        let Some(pending) = &mut self.files.pending else {
            return;
        };
        if let Operation::Import(category) = &mut pending.operation {
            pending
                .dialog
                .update_with_right_panel_ui(ctx, &mut |ui, _| {
                    import_options(ui, &self.document, category);
                });
        } else {
            pending.dialog.update(ctx);
        }
        let Some(path) = pending.dialog.take_picked() else {
            if matches!(
                pending.dialog.state(),
                egui_file_dialog::DialogState::Closed | egui_file_dialog::DialogState::Cancelled
            ) {
                self.files.pending = None;
            }
            return;
        };
        let operation = pending.operation;
        self.files.pending = None;
        let result = match operation {
            Operation::Import(category) => std::fs::read_to_string(path)
                .map_err(|error| error.to_string())
                .and_then(|text| self.import_calendar(&text, category)),
            Operation::Export => crate::ical::export(&self.document)
                .map_err(|error| error.to_string())
                .and_then(|text| std::fs::write(path, text).map_err(|error| error.to_string())),
        };
        if let Err(error) = result {
            self.notice = Some(error);
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) fn file_dialog(&mut self, ctx: &egui::Context) {
        if let Some(result) = self.files.browser.take_result() {
            if let Some(category) = self.files.import_category.take() {
                match result.and_then(|text| self.import_calendar(&text, category)) {
                    Ok(()) => {}
                    Err(error) => self.notice = Some(error),
                }
            }
            return;
        }
        let Some(category) = &mut self.files.import_category else {
            return;
        };
        if self.files.browser.is_reading() {
            return;
        }
        let language = self.document.language;
        let mut close = false;
        let mut pick = false;
        let title = language.text("Импорт iCalendar", "Import iCalendar");
        Dialog::Import.window(ctx, title).show(ctx, |ui| {
            close = Heading::new(title, language)
                .icon(icons::FILE_IMPORT, title)
                .show(ui);
            import_options(ui, &self.document, category);
            pick = ui
                .button(language.text("Выбрать .ics файл", "Choose .ics file"))
                .clicked();
        });
        if close {
            self.files.cancel_import();
        } else if pick && let Err(error) = self.files.browser.pick(ctx.clone()) {
            self.notice = Some(error);
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod browser;

#[cfg(test)]
mod tests;
