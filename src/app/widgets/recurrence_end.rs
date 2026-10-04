use std::num::NonZeroU32;

use chrono::NaiveDate;
use egui::{Id, Response, Ui, Widget};

use crate::{
    model::{InputError, Recurrence},
    text::Language,
};

/// Варианты исключают одновременное ограничение по дате и количеству; D задаёт черновик или разобранную дату.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::app) enum RecurrenceEnd<D> {
    #[default]
    Never,
    Until(D),
    Count(NonZeroU32),
}

pub(in crate::app) type RecurrenceEndDraft = RecurrenceEnd<String>;

impl RecurrenceEndDraft {
    pub(in crate::app) fn from_rule(rule: Option<Recurrence>) -> Self {
        rule.and_then(Recurrence::until).map_or_else(
            || {
                rule.and_then(Recurrence::count)
                    .map_or(Self::Never, Self::Count)
            },
            |date| Self::Until(date.to_string()),
        )
    }

    pub(in crate::app) fn parse(&self) -> Result<RecurrenceEnd<NaiveDate>, InputError> {
        match self {
            Self::Never => Ok(RecurrenceEnd::Never),
            Self::Count(count) => Ok(RecurrenceEnd::Count(*count)),
            Self::Until(text) => super::parse_date(text)
                .map(RecurrenceEnd::Until)
                .map_err(|_| InputError::RecurrenceUntil),
        }
    }
}

impl<D: Copy> RecurrenceEnd<D> {
    pub(in crate::app) const fn until(self) -> Option<D> {
        match self {
            Self::Until(date) => Some(date),
            Self::Never | Self::Count(_) => None,
        }
    }
    pub(in crate::app) const fn count(self) -> Option<NonZeroU32> {
        match self {
            Self::Count(count) => Some(count),
            Self::Never | Self::Until(_) => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Never,
    Until,
    Count,
}

impl Mode {
    const fn label(self, language: Language) -> &'static str {
        match self {
            Self::Never => language.text("Без окончания", "No end"),
            Self::Until => language.text("До даты", "Until date"),
            Self::Count => language.text("После N повторений", "After N occurrences"),
        }
    }
}

pub(in crate::app) struct RecurrenceEndInput<'a, F> {
    end: &'a mut RecurrenceEndDraft,
    id: Id,
    language: Language,
    default_date: F,
}

impl<'a, F: FnOnce() -> NaiveDate> RecurrenceEndInput<'a, F> {
    /// Начальная дата вычисляется только при переключении на ограничение по дате.
    pub(in crate::app) const fn new(
        end: &'a mut RecurrenceEndDraft,
        id: Id,
        language: Language,
        default_date: F,
    ) -> Self {
        Self {
            end,
            id,
            language,
            default_date,
        }
    }
}

impl<F: FnOnce() -> NaiveDate> Widget for RecurrenceEndInput<'_, F> {
    fn ui(self, ui: &mut Ui) -> Response {
        let mut changed = false;
        let mut response = ui
            .push_id(self.id, |ui| {
                ui.vertical(|ui| {
                    let mut mode = match self.end {
                        RecurrenceEnd::Never => Mode::Never,
                        RecurrenceEnd::Until(_) => Mode::Until,
                        RecurrenceEnd::Count(_) => Mode::Count,
                    };
                    if super::choice(
                        ui,
                        "mode",
                        &mut mode,
                        [Mode::Never, Mode::Until, Mode::Count],
                        |mode| mode.label(self.language),
                    )
                    .changed()
                    {
                        *self.end = match mode {
                            Mode::Never => RecurrenceEnd::Never,
                            Mode::Until => RecurrenceEnd::Until((self.default_date)().to_string()),
                            Mode::Count => RecurrenceEnd::Count(NonZeroU32::MIN),
                        };
                        changed = true;
                    }
                    match self.end {
                        RecurrenceEnd::Never => {}
                        RecurrenceEnd::Until(text) => {
                            changed |= ui
                                .add(super::DateInput::new(
                                    text,
                                    self.id.with("until"),
                                    self.language.text("Конец повторений", "Repeat until"),
                                    self.language,
                                ))
                                .changed();
                        }
                        RecurrenceEnd::Count(count) => {
                            let mut value = count.get();
                            let response =
                                ui.add(egui::DragValue::new(&mut value).range(1..=u32::MAX));
                            response.widget_info(|| {
                                egui::WidgetInfo::labeled(
                                    egui::WidgetType::DragValue,
                                    response.enabled(),
                                    self.language
                                        .text("Количество повторений", "Occurrence count"),
                                )
                            });
                            if response.changed() {
                                *count = NonZeroU32::new(value)
                                    .expect("Occurrence count input is bounded");
                                changed = true;
                            }
                        }
                    }
                })
                .response
            })
            .inner;
        if changed {
            response.mark_changed();
        }
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Frequency;
    use std::num::NonZeroU16;

    #[test]
    fn end_modes_round_trip_and_until_requires_a_complete_date() {
        let date = "2026-10-04".parse().unwrap();
        let rule = Recurrence::new(Frequency::Daily, NonZeroU16::MIN, Some(date)).unwrap();
        let end = RecurrenceEndDraft::from_rule(Some(rule)).parse().unwrap();
        assert_eq!(end.until(), Some(date));
        assert_eq!(end.count(), None);
        let count = NonZeroU32::new(10).unwrap();
        let rule = Recurrence::new(Frequency::Daily, NonZeroU16::MIN, None)
            .unwrap()
            .with_pattern(Some(count), None, chrono::Weekday::Mon, None)
            .unwrap();
        let end = RecurrenceEndDraft::from_rule(Some(rule)).parse().unwrap();
        assert_eq!(end.until(), None);
        assert_eq!(end.count(), Some(count));
        assert!(RecurrenceEndDraft::Until(String::new()).parse().is_err());
        assert!(
            RecurrenceEndDraft::Until("31.02.2026".into())
                .parse()
                .is_err()
        );
        assert_eq!(RecurrenceEndDraft::Never.parse().unwrap().until(), None);
    }
}
