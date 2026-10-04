use egui::{Event, Key, Modifiers};

use super::Planner;

#[derive(Clone, Copy)]
pub(super) enum Shortcut {
    Search,
    NewEvent,
    Today,
    PreviousYear,
    NextYear,
    Cancel,
    SaveEvent,
}

/// Повторная компоновка кадра повторяет потребление ввода, но не само действие.
#[derive(Default)]
pub(super) struct Shortcuts {
    frame: Option<Frame>,
}

struct Frame {
    number: u64,
    consumed: Option<Shortcut>,
    popup_open: bool,
}

impl Shortcuts {
    fn begin_pass(&mut self, ctx: &egui::Context) -> bool {
        let number = ctx.cumulative_frame_nr();
        if let Some(frame) = &self.frame
            && frame.number == number
        {
            if let Some(shortcut) = frame.consumed {
                shortcut.consume(ctx);
            }
            return false;
        }
        self.frame = Some(Frame {
            number,
            consumed: None,
            popup_open: egui::Popup::is_any_open(ctx),
        });
        true
    }

    pub(super) fn popup_open(&self, ctx: &egui::Context) -> bool {
        self.frame
            .as_ref()
            .is_some_and(|frame| frame.number == ctx.cumulative_frame_nr() && frame.popup_open)
            || egui::Popup::is_any_open(ctx)
    }

    pub(super) fn consume(&mut self, shortcut: Shortcut, ctx: &egui::Context) -> bool {
        let pressed = shortcut.consume(ctx);
        if pressed && let Some(frame) = &mut self.frame {
            frame.consumed = Some(shortcut);
        }
        pressed
    }
}

impl Shortcut {
    fn consume(self, ctx: &egui::Context) -> bool {
        let (modifiers, key) = match self {
            Self::Search => (Modifiers::COMMAND, Key::K),
            Self::NewEvent => (Modifiers::NONE, Key::N),
            Self::Today => (Modifiers::NONE, Key::T),
            Self::PreviousYear => (Modifiers::NONE, Key::PageUp),
            Self::NextYear => (Modifiers::NONE, Key::PageDown),
            Self::Cancel => (Modifiers::NONE, Key::Escape),
            Self::SaveEvent => (Modifiers::COMMAND, Key::Enter),
        };
        ctx.input_mut(|input| {
            let mut pressed = false;
            let matches_key = |event: &Event| {
                matches!(event, Event::Key {
                    key: event_key,
                    modifiers: event_modifiers,
                    pressed: true,
                    ..
                } if *event_key == key && event_modifiers.matches_exact(modifiers))
            };
            let letters: &[&str] = match self {
                Self::NewEvent => &["n", "N", "т", "Т"],
                Self::Today => &["t", "T", "е", "Е"],
                _ => &[],
            };
            // Desktop передаёт символ после Key, web — перед ним; оба события относятся к одной команде.
            let consume_text = !letters.is_empty() && input.events.iter().any(matches_key);
            input.events.retain(|event| match event {
                Event::Key { repeat, .. } if matches_key(event) => {
                    // Автоповтор не должен закрывать следующий слой интерфейса или повторять действие.
                    pressed |= !repeat;
                    false
                }
                // Символ команды не должен попасть в поле открываемого редактора.
                Event::Text(text) if consume_text && letters.contains(&text.as_str()) => false,
                _ => true,
            });
            pressed
        })
    }
}

impl Planner {
    pub(super) fn shortcuts(&mut self, ctx: &egui::Context) {
        if !self.shortcuts.begin_pass(ctx)
            || self.shortcuts_blocked()
            || self.shortcuts.popup_open(ctx)
            || !ctx.input(|input| {
                input
                    .events
                    .iter()
                    .any(|event| matches!(event, Event::Key { pressed: true, .. }))
            })
        {
            return;
        }
        if self.shortcuts.consume(Shortcut::Search, ctx) {
            self.open_search();
            return;
        }
        if self.search.open {
            if self.shortcuts.consume(Shortcut::Cancel, ctx) {
                self.search.open = false;
            }
            return;
        }
        if ctx.text_edit_focused() {
            return;
        }
        if self.shortcuts.consume(Shortcut::Cancel, ctx) {
            self.selection.clear();
        } else if self.shortcuts.consume(Shortcut::NewEvent, ctx) {
            self.new_event();
        } else if self.shortcuts.consume(Shortcut::Today, ctx) {
            self.go_to_today();
        } else {
            let delta = if self.shortcuts.consume(Shortcut::PreviousYear, ctx) {
                -1
            } else if self.shortcuts.consume(Shortcut::NextYear, ctx) {
                1
            } else {
                return;
            };
            if let Some(year) = self.document.year.step(delta) {
                self.change_year(year);
            }
        }
    }

    pub(super) const fn shortcuts_blocked(&self) -> bool {
        self.event_draft.is_some()
            || self.tree_draft.is_some()
            || self.backup.is_some()
            || self.about_open
            || self.notice.is_some()
            || self.vacation.open
            || self.year_draft.is_some()
            || self.rename_draft.is_some()
            || self.files.dialog_open()
    }
}
