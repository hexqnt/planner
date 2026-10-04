use chrono::Weekday;
use egui::{Response, Ui, Widget};

use crate::{model::Weekdays, text::Language};

pub(in crate::app) struct WeekdaySelector<'a> {
    days: &'a mut Weekdays,
    language: Language,
}

impl<'a> WeekdaySelector<'a> {
    pub(in crate::app) const fn new(days: &'a mut Weekdays, language: Language) -> Self {
        Self { days, language }
    }
}

impl Widget for WeekdaySelector<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let before = *self.days;
        let mut response = ui
            .horizontal_wrapped(|ui| {
                for (day, ru, en, full_ru, full_en) in [
                    (Weekday::Mon, "Пн", "Mo", "Понедельник", "Monday"),
                    (Weekday::Tue, "Вт", "Tu", "Вторник", "Tuesday"),
                    (Weekday::Wed, "Ср", "We", "Среда", "Wednesday"),
                    (Weekday::Thu, "Чт", "Th", "Четверг", "Thursday"),
                    (Weekday::Fri, "Пт", "Fr", "Пятница", "Friday"),
                    (Weekday::Sat, "Сб", "Sa", "Суббота", "Saturday"),
                    (Weekday::Sun, "Вс", "Su", "Воскресенье", "Sunday"),
                ] {
                    let selected = self.days.contains(day);
                    let response = ui.selectable_label(selected, self.language.text(ru, en));
                    let response = response.on_hover_text(self.language.text(full_ru, full_en));
                    if response.clicked() {
                        *self.days = toggle(*self.days, day);
                    }
                }
            })
            .response;
        if before != *self.days {
            response.mark_changed();
        }
        response
    }
}

fn toggle(days: Weekdays, day: Weekday) -> Weekdays {
    Weekdays::try_from(u8::from(days) ^ (1 << day.num_days_from_monday())).unwrap_or(days)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn last_selected_day_cannot_be_removed() {
        let monday = Weekdays::try_from(1).unwrap();
        assert_eq!(toggle(monday, Weekday::Mon), monday);
        assert_eq!(u8::from(toggle(monday, Weekday::Tue)), 3);
        assert_eq!(
            toggle(toggle(monday, Weekday::Tue), Weekday::Mon),
            Weekdays::try_from(2).unwrap()
        );
    }
}
