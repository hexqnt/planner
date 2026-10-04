use egui::{Event, Id, Key, Modifiers, Response, TextEdit, Ui, Widget, text::CCursorRange};

use crate::text::Language;

/// Действие меню выполняется перед следующей отрисовкой поля штатным обработчиком egui, сохраняя выделение и историю отмены.
#[derive(Clone)]
struct PendingAction {
    id: Id,
    selection: Option<CCursorRange>,
    event: Event,
}

fn pending_id() -> Id {
    Id::new("text_edit_menu_action")
}

pub(in crate::app) fn apply_pending_action(ctx: &egui::Context) {
    if let Some(action) = ctx.data_mut(|data| {
        data.remove_temp::<Option<PendingAction>>(pending_id())
            .flatten()
    }) {
        ctx.memory_mut(|memory| memory.request_focus(action.id));
        if let Some(mut state) = TextEdit::load_state(ctx, action.id) {
            state.cursor.set_char_range(action.selection);
            state.store(ctx, action.id);
        }
        ctx.input_mut(|input| input.events.push(action.event));
    }
}

pub(in crate::app) trait TextEditExt<'a> {
    fn with_context_menu(self, language: Language) -> TextEditMenu<'a>;
    fn show_with_context_menu(
        self,
        ui: &mut Ui,
        language: Language,
    ) -> egui::text_edit::TextEditOutput;
}

impl<'a> TextEditExt<'a> for TextEdit<'a> {
    fn with_context_menu(self, language: Language) -> TextEditMenu<'a> {
        TextEditMenu {
            edit: self,
            language,
        }
    }

    fn show_with_context_menu(
        self,
        ui: &mut Ui,
        language: Language,
    ) -> egui::text_edit::TextEditOutput {
        let preserved = ui
            .input(|input| input.pointer.button_pressed(egui::PointerButton::Secondary))
            .then(|| {
                let id = ui.memory(egui::Memory::focused)?;
                let state = TextEdit::load_state(ui.ctx(), id)?;
                Some((id, state.cursor.char_range()))
            })
            .flatten();
        let mut output = self.show(ui);
        if let Some((id, selection)) = preserved
            && id == output.response.id
            && output.response.hovered()
        {
            // egui перемещает курсор при нажатии любой кнопки мыши; меню должно сохранять выделение правого клика.
            output.state.cursor.set_char_range(selection);
            output.cursor_range = selection;
            output.state.clone().store(ui.ctx(), id);
        }
        output
            .response
            .context_menu(|ui| menu_contents(ui, &output, language));
        output
    }
}

fn menu_contents(ui: &mut Ui, output: &egui::text_edit::TextEditOutput, language: Language) {
    let selection = output.state.cursor.char_range();
    let has_selection = selection.is_some_and(|range| !range.is_empty());
    let has_text = !output.galley.text().is_empty();
    // Клик по меню не должен завершать переименование или редактирование года.
    output.response.request_focus();
    let undoer = output.state.undoer();
    let current = (
        selection.unwrap_or_default(),
        output.galley.text().to_owned(),
    );
    let key = |key, modifiers| Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    };
    for (label, enabled, event) in [
        (
            language.text("Отменить", "Undo"),
            undoer.has_undo(&current),
            key(Key::Z, Modifiers::COMMAND),
        ),
        (
            language.text("Повторить", "Redo"),
            undoer.has_redo(&current),
            key(Key::Z, Modifiers::COMMAND | Modifiers::SHIFT),
        ),
        (language.text("Вырезать", "Cut"), has_selection, Event::Cut),
        (
            language.text("Копировать", "Copy"),
            has_selection,
            Event::Copy,
        ),
        (
            language.text("Вставить", "Paste"),
            true,
            key(Key::Paste, Modifiers::NONE),
        ),
        (
            language.text("Удалить выделенное", "Delete selection"),
            has_selection,
            key(Key::Delete, Modifiers::NONE),
        ),
        (
            language.text("Выделить всё", "Select all"),
            has_text,
            key(Key::A, Modifiers::COMMAND),
        ),
    ] {
        if ui.add_enabled(enabled, egui::Button::new(label)).clicked() {
            output.response.request_focus();
            let mut state = output.state.clone();
            if matches!(
                event,
                Event::Cut
                    | Event::Key {
                        key: Key::Delete | Key::Paste,
                        ..
                    }
            ) {
                // Действие меню образует отдельный шаг отмены даже сразу после ввода текста.
                let mut undoer = state.undoer();
                undoer.add_undo(&current);
                state.set_undoer(undoer);
            }
            state.store(ui.ctx(), output.response.id);
            if matches!(
                event,
                Event::Key {
                    key: Key::Paste,
                    ..
                }
            ) {
                request_paste(ui.ctx(), output.response.id, selection);
            } else {
                ui.ctx().data_mut(|data| {
                    data.insert_temp(
                        pending_id(),
                        Some(PendingAction {
                            id: output.response.id,
                            selection,
                            event,
                        }),
                    )
                });
            }
            ui.close();
            ui.ctx().request_repaint();
        }
    }
}

pub(in crate::app) struct TextEditMenu<'a> {
    edit: TextEdit<'a>,
    language: Language,
}

impl Widget for TextEditMenu<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        self.edit
            .show_with_context_menu(ui, self.language)
            .response
            .response
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn request_paste(ctx: &egui::Context, _id: Id, _selection: Option<CCursorRange>) {
    ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste);
}

#[cfg(target_arch = "wasm32")]
fn request_paste(ctx: &egui::Context, id: Id, selection: Option<CCursorRange>) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let promise = window.navigator().clipboard().read_text();
    let ctx = ctx.clone();
    wasm_bindgen_futures::spawn_local(async move {
        match wasm_bindgen_futures::JsFuture::from(promise).await {
            Ok(value) => {
                if let Some(text) = value.as_string()
                    && ctx.memory(egui::Memory::focused) == Some(id)
                {
                    ctx.data_mut(|data| {
                        data.insert_temp(
                            pending_id(),
                            Some(PendingAction {
                                id,
                                selection,
                                event: Event::Paste(text),
                            }),
                        )
                    });
                    ctx.request_repaint();
                }
            }
            Err(error) => web_sys::console::error_2(&"Unable to read clipboard".into(), &error),
        }
    });
}
