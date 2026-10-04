//! Общие элементы оформления; действия и состояние остаются у вызывающего компонента.

use egui::{Button, Color32, Response, RichText, TextBuffer, TextEdit, Ui};

use super::ui_config::style;
use crate::{model::DateRange, text::Language};

mod action_button;
mod calendar_color;
mod calendar_picker;
mod choice;
mod date_time;
mod filter_chip;
mod form_row;
mod help;
mod money;
mod parsed_text;
mod recurrence_end;
mod reminder;
mod search_input;
mod text_edit;
mod weekday_selector;
mod year_input;

pub(super) use action_button::ActionButton;
pub(super) use calendar_color::CalendarColorInput;
pub(super) use calendar_picker::CalendarPicker;
pub(super) use choice::choice;
pub(super) use date_time::{DateInput, TimeInput, format_time, parse_date, parse_time};
pub(super) use filter_chip::FilterChip;
pub(super) use form_row::FormRow;
pub(super) use help::Help;
pub(super) use money::{MoneyDraft, MoneyInput};
pub(super) use parsed_text::{EmailListDraft, EmailListInput, LinkDraft, LinkInput};
pub(super) use recurrence_end::{RecurrenceEndDraft, RecurrenceEndInput};
pub(super) use reminder::ReminderInput;
pub(super) use search_input::SearchInput;
pub(super) use text_edit::{TextEditExt, apply_pending_action};
pub(super) use weekday_selector::WeekdaySelector;
pub(super) use year_input::{YearDraft, YearInput};

/// Подпись доступна без наведения, в том числе для кнопок с одной иконкой.
pub(super) fn labelled_action(response: Response, label: &str) -> Response {
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, response.enabled(), label)
    });
    response.on_hover_text(label)
}

pub(super) fn hint(response: Response, text: &str) -> Response {
    let width = style::HELP_MAX_WIDTH.min(
        2.0_f32
            .mul_add(
                -style::HELP_SCREEN_MARGIN,
                response.ctx.content_rect().width(),
            )
            .max(1.0),
    );
    response.on_hover_ui(|ui| {
        ui.set_width(width);
        ui.add(egui::Label::new(text).wrap());
    })
}

pub(super) fn primary_button(ui: &mut Ui, label: &str) -> Response {
    ui.add(
        Button::new(RichText::new(label).color(style::TEXT_ON_ACCENT))
            .fill(ui.visuals().selection.bg_fill),
    )
}

pub(super) fn text_input<'a>(ui: &Ui, text: &'a mut dyn TextBuffer) -> TextEdit<'a> {
    TextEdit::singleline(text).margin(ui.spacing().button_padding)
}

pub(super) fn calendar_marker([r, g, b]: [u8; 3]) -> RichText {
    RichText::new("●").color(Color32::from_rgb(r, g, b))
}

pub(super) fn error_label(ui: &mut Ui, text: &str) -> Response {
    ui.colored_label(ui.visuals().error_fg_color, text)
}

pub(super) fn warning_label(ui: &mut Ui, text: &str) -> Response {
    ui.colored_label(ui.visuals().warn_fg_color, text)
}

pub(super) fn date_range(ui: &mut Ui, range: DateRange, language: Language) -> Response {
    ui.label(format!(
        "{range} · {} {}",
        range.days(),
        language.text("дн.", "days")
    ))
}

pub(super) fn form_text_frame(ui: &egui::Ui) -> egui::Frame {
    egui::Frame::NONE.inner_margin(egui::vec2(0.0, ui.spacing().button_padding.y))
}

pub(super) fn form_input<'a>(ui: &Ui, text: &'a mut dyn TextBuffer) -> TextEdit<'a> {
    TextEdit::singleline(text)
        .frame(form_text_frame(ui))
        .desired_width(f32::INFINITY)
}
