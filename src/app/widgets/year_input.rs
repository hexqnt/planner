use std::ops::Range;

use egui::{Key, Modifiers, Response, TextBuffer, Ui, Vec2, Widget, text::CharIndex};

use super::TextEditExt as _;
use crate::{
    app::ui_config::layout::{YEAR_FONT_SIZE, YEAR_STEP_BUTTON_SIZE},
    model::Year,
    text::Language,
};

pub(in crate::app) struct YearDraft {
    text: Digits,
    request_focus: bool,
}

impl YearDraft {
    fn new(year: Year) -> Self {
        Self {
            text: Digits(year.get().to_string()),
            request_focus: true,
        }
    }
}

// Буфер содержит только ASCII-цифры, поэтому индексы символов совпадают с байтовыми.
struct Digits(String);

impl Digits {
    fn year(&self) -> Option<Year> {
        if self.0.is_empty() {
            return None;
        }
        let value = self.0.bytes().fold(0_i32, |value, digit| {
            value
                .saturating_mul(10)
                .saturating_add(i32::from(digit - b'0'))
        });
        Some(Year::clamped(value))
    }
}

impl TextBuffer for Digits {
    fn type_id(&self) -> std::any::TypeId {
        std::any::TypeId::of::<Self>()
    }
    fn is_mutable(&self) -> bool {
        true
    }
    fn as_str(&self) -> &str {
        &self.0
    }

    fn insert_text(&mut self, text: &str, char_index: CharIndex) -> usize {
        let start = char_index.0.min(self.0.len());
        let mut inserted = 0;
        for digits in text.split(|character: char| !character.is_ascii_digit()) {
            if !digits.is_empty() {
                self.0.insert_str(start + inserted, digits);
                inserted += digits.len();
            }
        }
        inserted
    }

    fn delete_char_range(&mut self, char_range: Range<CharIndex>) {
        self.0.delete_char_range(char_range);
    }
}

pub(in crate::app) struct YearInput<'a> {
    year: &'a mut Year,
    draft: &'a mut Option<YearDraft>,
    language: Language,
}

impl<'a> YearInput<'a> {
    pub(in crate::app) const fn new(
        year: &'a mut Year,
        draft: &'a mut Option<YearDraft>,
        language: Language,
    ) -> Self {
        Self {
            year,
            draft,
            language,
        }
    }

    fn step(&mut self, delta: i32) {
        if let Some(year) = self.year.step(delta) {
            *self.year = year;
            *self.draft = None;
        }
    }

    fn step_button(&mut self, ui: &mut Ui, delta: i32, label: &str) {
        // Место под скрытую на границе стрелку сохраняется, чтобы год оставался по центру.
        if ui
            .add_visible(
                self.year.step(delta).is_some(),
                egui::Button::new(label).min_size(YEAR_STEP_BUTTON_SIZE),
            )
            .on_hover_text(if delta < 0 {
                self.language
                    .text("Предыдущий год · PageUp", "Previous year · PageUp")
            } else {
                self.language
                    .text("Следующий год · PageDown", "Next year · PageDown")
            })
            .clicked()
        {
            self.step(delta);
        }
    }

    fn year_field(&mut self, ui: &mut Ui, font: egui::FontId, width: f32) {
        let Some(draft) = self.draft.as_mut() else {
            let label = ui.painter().layout_no_wrap(
                self.year.get().to_string(),
                font,
                ui.visuals().text_color(),
            );
            let (rect, response) =
                ui.allocate_exact_size(Vec2::new(width, label.size().y), egui::Sense::click());
            response.widget_info(|| {
                egui::WidgetInfo::labeled(
                    egui::WidgetType::Button,
                    response.enabled(),
                    label.text(),
                )
            });
            let response = response.on_hover_text(self.language.text(
                "Двойной клик — ввод года · Колёсико — смена года",
                "Double-click to enter a year · Scroll to change the year",
            ));
            ui.painter().galley(
                rect.center() - label.size() / 2.0,
                label,
                ui.visuals().text_color(),
            );
            if response.double_clicked() {
                *self.draft = Some(YearDraft::new(*self.year));
                ui.ctx().request_repaint();
            }
            return;
        };
        let (escape, enter) = ui.input(|input| {
            (
                input.key_pressed(Key::Escape),
                input.key_pressed(Key::Enter),
            )
        });
        let output = egui::TextEdit::singleline(&mut draft.text)
            .id_salt("year_editor")
            .font(font)
            .desired_width(width)
            .margin(egui::Margin::ZERO)
            .show_with_context_menu(ui, self.language);
        let response = output.response;
        response.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::TextEdit,
                response.enabled(),
                self.language.text("Год", "Year"),
            )
        });
        if std::mem::take(&mut draft.request_focus) {
            response.request_focus();
            let mut state = output.state;
            state
                .cursor
                .set_char_range(Some(egui::text::CCursorRange::two(
                    egui::text::CCursor::new(0),
                    egui::text::CCursor::new(draft.text.0.len()),
                )));
            state.store(ui.ctx(), response.id);
            ui.ctx().request_repaint();
        }
        let active = response.has_focus() || response.lost_focus();
        let cancel = escape && active;
        let accept = (enter && active) || (response.lost_focus() && !response.has_focus());
        if !cancel && !accept {
            return;
        }
        if active && (escape || enter) {
            ui.input_mut(|input| {
                input.consume_key(
                    Modifiers::NONE,
                    if cancel { Key::Escape } else { Key::Enter },
                )
            });
        }
        response.surrender_focus();
        let draft = self.draft.take().expect("Year editor is open");
        if !cancel && let Some(year) = draft.text.year() {
            *self.year = year;
        }
    }
}

impl Widget for YearInput<'_> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        let before = *self.year;
        let rect = ui.available_rect_before_wrap();
        let mut font = egui::TextStyle::Heading.resolve(ui.style());
        font.size = YEAR_FONT_SIZE;
        let text_width = year_text_width(ui, &font);
        let width = ui
            .spacing()
            .item_spacing
            .x
            .mul_add(2.0, YEAR_STEP_BUTTON_SIZE.x.mul_add(2.0, text_width));
        let center = egui::Rect::from_center_size(rect.center(), Vec2::new(width, rect.height()));
        if ui.is_enabled() && self.draft.is_none() && ui.rect_contains_pointer(center) {
            let scroll = ui.input(|input| {
                input
                    .events
                    .iter()
                    .filter_map(|event| match event {
                        egui::Event::MouseWheel { delta, .. } => Some(delta.y),
                        _ => None,
                    })
                    .sum::<f32>()
            });
            if scroll != 0.0 {
                self.step(if scroll > 0.0 { 1 } else { -1 });
            }
        }
        let mut response = ui
            .scope_builder(
                egui::UiBuilder::new()
                    .max_rect(center)
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
                |ui| {
                    self.step_button(ui, -1, "<");
                    self.year_field(ui, font, text_width);
                    self.step_button(ui, 1, ">");
                },
            )
            .response;
        if before != *self.year {
            response.mark_changed();
            ui.ctx().request_repaint();
        }
        response
    }
}

/// Фиксированная ширина четырёх цифр не даёт боковым кнопкам смещаться при смене года.
fn year_text_width(ui: &Ui, font: &egui::FontId) -> f32 {
    let widest = ui.painter().fonts_mut(|fonts| {
        b"0123456789"
            .iter()
            .map(|&digit| fonts.glyph_width(font, char::from(digit)))
            .fold(0.0_f32, f32::max)
    });
    widest * 4.0
}

#[cfg(test)]
mod tests;
