use egui::{Id, Key, Modifiers, Response, Ui, Widget};

use super::TextEditExt as _;
use crate::text::Language;

mod value;

use value::Value;
pub(in crate::app) use value::{format_time, parse_date, parse_time};

/// Состояние редактирования хранится в egui и не попадает в документ.
#[derive(Clone, Copy, Default)]
struct InputState {
    committed: bool,
    language: Option<Language>,
    wheel_active: bool,
}

#[derive(Clone, Copy)]
enum Kind {
    Date,
    Time,
}

impl Kind {
    fn parse(self, text: &str) -> Option<Value> {
        match self {
            Self::Date => parse_date(text).ok().map(Value::Date),
            Self::Time => parse_time(text).ok().map(Value::Time),
        }
    }

    const fn hint(self, language: Language) -> &'static str {
        match self {
            Self::Date => language.text("ДД.ММ.ГГГГ", "YYYY-MM-DD"),
            Self::Time => "HH:MM",
        }
    }

    const fn error(self, language: Language) -> &'static str {
        match self {
            Self::Date => language.text("Некорректная дата", "Invalid date"),
            Self::Time => language.text("Некорректное время", "Invalid time"),
        }
    }
}

pub(in crate::app) struct DateInput<'a>(Input<'a>);
pub(in crate::app) struct TimeInput<'a>(Input<'a>);

impl<'a> DateInput<'a> {
    pub(in crate::app) const fn new(
        text: &'a mut String,
        id: Id,
        label: &'a str,
        language: Language,
    ) -> Self {
        Self(Input {
            text,
            id,
            label,
            language,
            kind: Kind::Date,
        })
    }
}

impl<'a> TimeInput<'a> {
    pub(in crate::app) const fn new(
        text: &'a mut String,
        id: Id,
        label: &'a str,
        language: Language,
    ) -> Self {
        Self(Input {
            text,
            id,
            label,
            language,
            kind: Kind::Time,
        })
    }
}

impl Widget for DateInput<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        self.0.show(ui)
    }
}

impl Widget for TimeInput<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        self.0.show(ui)
    }
}

struct Input<'a> {
    text: &'a mut String,
    id: Id,
    label: &'a str,
    language: Language,
    kind: Kind,
}

impl Input<'_> {
    fn show(mut self, ui: &mut Ui) -> Response {
        let state_id = self.id.with("date_time_input");
        let mut state =
            ui.data_mut(|data| data.get_temp::<InputState>(state_id).unwrap_or_default());
        let focused = ui.memory(|memory| memory.has_focus(self.id));
        let mut value = self.kind.parse(self.text);
        if !focused
            && state.language != Some(self.language)
            && let Some(value) = value
        {
            *self.text = value.format(self.language);
        }
        let hovered = ui
            .ctx()
            .read_response(self.id)
            .is_some_and(|response| response.enabled() && ui.rect_contains_pointer(response.rect));
        let wheel = wheel_direction(ui, focused && hovered && value.is_some(), &mut state);
        let direction = focused
            .then(|| {
                ui.input_mut(|input| {
                    if input.consume_key(Modifiers::NONE, Key::ArrowUp) {
                        Some(true)
                    } else if input.consume_key(Modifiers::NONE, Key::ArrowDown) {
                        Some(false)
                    } else {
                        None
                    }
                })
            })
            .flatten()
            .or(wheel);
        let next = direction.and_then(|forward| self.step(ui, value, forward));
        let stepped = next.is_some();
        value = next.or(value);
        ui.vertical(|ui| {
            let width = match self.kind {
                Kind::Date => crate::app::ui_config::event::DATE_WIDTH,
                Kind::Time => crate::app::ui_config::event::TIME_WIDTH,
            };
            ui.set_max_width(width);
            let mut response = ui.add(
                super::text_input(ui, self.text)
                    .id(self.id)
                    .desired_width(width)
                    .hint_text(self.kind.hint(self.language))
                    .with_context_menu(self.language),
            );
            response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, response.enabled(), self.label)
            });
            if response.changed() {
                value = self.kind.parse(self.text);
                state.committed = false;
            }
            if stepped {
                response.mark_changed();
            }
            if response.lost_focus() {
                state.committed = true;
                if let Some(value) = value {
                    let normalized = value.format(self.language);
                    if *self.text != normalized {
                        *self.text = normalized;
                        response.mark_changed();
                    }
                }
            }
            if state.committed && value.is_none() {
                let error = egui::RichText::new(self.kind.error(self.language))
                    .color(ui.visuals().error_fg_color);
                ui.add(egui::Label::new(error).wrap());
            }
            state.language = Some(self.language);
            ui.data_mut(|data| data.insert_temp(state_id, state));
            response.on_hover_text(self.language.text(
                "↑/↓ или колёсико над активным полем — изменить часть под курсором. Дата: ДД.ММ.ГГГГ или ГГГГ-ММ-ДД. Время: ЧЧ:ММ, 930 или с секундами.",
                "↑/↓ or the mouse wheel over the focused field adjusts the part at the cursor. Date: YYYY-MM-DD or DD.MM.YYYY. Time: HH:MM, 930 or with seconds.",
            ))
        }).inner
    }

    fn step(&mut self, ui: &Ui, value: Option<Value>, forward: bool) -> Option<Value> {
        let current = value?;
        let mut editor = egui::TextEdit::load_state(ui.ctx(), self.id)?;
        let cursor = editor
            .cursor
            .char_range()
            .map_or(0, |range| range.primary.index.into());
        let next = current.step(self.text, cursor, forward)?;
        // Порядок частей даты сохраняется до выхода из поля, чтобы курсор не менял смысл.
        let language = match next {
            Value::Date(_) if self.text.contains('-') => Language::English,
            Value::Date(_) => Language::Russian,
            Value::Time(_) => self.language,
        };
        let formatted = next.format(language);
        if matches!(next, Value::Time(_))
            && !self.text.contains(':')
            && cursor > self.text.len().saturating_sub(2)
        {
            // В короткой записи минутам соответствуют последние две цифры.
            let index = cursor + formatted.len().saturating_sub(self.text.len());
            editor
                .cursor
                .set_char_range(Some(egui::text::CCursorRange::one(
                    egui::text::CCursor::new(index),
                )));
            editor.store(ui.ctx(), self.id);
        }
        *self.text = formatted;
        Some(next)
    }
}

fn wheel_direction(ui: &Ui, enabled: bool, state: &mut InputState) -> Option<bool> {
    ui.input_mut(|input| {
        if input
            .events
            .iter()
            .any(|event| matches!(event, egui::Event::MouseWheel { delta, .. } if delta.y != 0.0))
        {
            state.wheel_active = false;
        }
        let mut delta = 0.0;
        if enabled {
            input.events.retain_mut(|event| {
                if let egui::Event::MouseWheel {
                    delta: scroll,
                    modifiers: Modifiers::NONE,
                    ..
                } = event
                    && scroll.y != 0.0
                    && scroll.y.is_finite()
                {
                    delta += scroll.y;
                    state.wheel_active = true;
                    scroll.y = 0.0;
                    scroll.x != 0.0
                } else {
                    true
                }
            });
        }
        if state.wheel_active {
            // Шаг определяется исходным событием; сглаженный хвост не должен прокручивать форму или повторять шаг.
            input.smooth_scroll_delta.y = 0.0;
            state.wheel_active = input.is_scrolling();
        }
        (delta != 0.0).then_some(delta > 0.0)
    })
}

#[cfg(test)]
mod tests;
