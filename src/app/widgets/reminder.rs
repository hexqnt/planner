use egui::{Id, Response, Ui, Widget};

use crate::{model::Reminder, text::Language};

#[derive(Clone, Copy, PartialEq)]
enum Unit {
    Minutes,
    Hours,
    Days,
}

impl Unit {
    const fn scale(self) -> u32 {
        match self {
            Self::Minutes => 1,
            Self::Hours => 60,
            Self::Days => 1440,
        }
    }
    const fn label(self, language: Language) -> &'static str {
        match self {
            Self::Minutes => language.text("мин.", "min."),
            Self::Hours => language.text("ч.", "hours"),
            Self::Days => language.text("дн.", "days"),
        }
    }
    const fn for_reminder(reminder: Reminder) -> Self {
        let minutes = reminder.minutes();
        if minutes != 0 && minutes.is_multiple_of(1440) {
            Self::Days
        } else if minutes != 0 && minutes.is_multiple_of(60) {
            Self::Hours
        } else {
            Self::Minutes
        }
    }
}

pub(in crate::app) struct ReminderInput<'a> {
    reminder: &'a mut Reminder,
    id: Id,
    language: Language,
}

impl<'a> ReminderInput<'a> {
    pub(in crate::app) const fn new(
        reminder: &'a mut Reminder,
        id: Id,
        language: Language,
    ) -> Self {
        Self {
            reminder,
            id,
            language,
        }
    }
}

impl Widget for ReminderInput<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let mut unit = ui.data_mut(|data| {
            data.get_temp::<Unit>(self.id)
                .unwrap_or_else(|| Unit::for_reminder(*self.reminder))
        });
        let before = *self.reminder;
        let mut response = ui
            .push_id(self.id, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(self.language.text("За", "Before"));
                    let mut minutes = self.reminder.minutes();
                    let scale = f64::from(unit.scale());
                    let response = ui.add(
                        egui::DragValue::new(&mut minutes)
                            .range(0..=Reminder::MAX_MINUTES)
                            .speed(scale)
                            .custom_formatter(move |minutes, _| format_quantity(minutes / scale))
                            .custom_parser(move |text| parse_quantity(text, scale)),
                    );
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(
                            egui::WidgetType::DragValue,
                            response.enabled(),
                            self.language
                                .text("Интервал напоминания", "Reminder interval"),
                        )
                    });
                    if response.changed() {
                        *self.reminder =
                            Reminder::try_from(minutes).expect("Reminder input is bounded");
                    }
                    super::choice(
                        ui,
                        "unit",
                        &mut unit,
                        [Unit::Minutes, Unit::Hours, Unit::Days],
                        |unit| unit.label(self.language),
                    );
                    if self.reminder.minutes() == 0 {
                        ui.weak(self.language.text("В момент начала", "At the start"));
                    }
                })
                .response
            })
            .inner;
        ui.data_mut(|data| data.insert_temp(self.id, unit));
        if before != *self.reminder {
            response.mark_changed();
        }
        super::hint(
            response,
            self.language.text(
                "Дробные часы и дни округляются до ближайшей минуты.",
                "Fractional hours and days are rounded to the nearest minute.",
            ),
        )
    }
}

fn format_quantity(quantity: f64) -> String {
    // Шести знаков достаточно, чтобы отображение дней сохраняло точность до минуты при повторном вводе.
    let mut text = format!("{quantity:.6}");
    let length = text.trim_end_matches('0').trim_end_matches('.').len();
    text.truncate(length);
    text
}

fn parse_quantity(text: &str, scale: f64) -> Option<f64> {
    let text = text.trim();
    let quantity: f64 = if text.contains(',') {
        text.replace(',', ".").parse()
    } else {
        text.parse()
    }
    .ok()?;
    let minutes = (quantity * scale).round();
    minutes.is_finite().then_some(minutes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn units_preserve_minute_precision_including_fractional_days() {
        for unit in [Unit::Minutes, Unit::Hours, Unit::Days] {
            let scale = f64::from(unit.scale());
            for minutes in [0, 1, 15, 60, 1441, Reminder::MAX_MINUTES] {
                assert_eq!(
                    parse_quantity(&format_quantity(f64::from(minutes) / scale), scale),
                    Some(f64::from(minutes))
                );
            }
        }
        assert_eq!(parse_quantity("1,5", 60.0), Some(90.0));
        assert!(parse_quantity("NaN", 1.0).is_none());
        assert!(parse_quantity("inf", 1440.0).is_none());
    }
}
