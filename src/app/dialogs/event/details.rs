use std::num::NonZeroU16;

use crate::{
    app::{icons, widgets, widgets::TextEditExt as _},
    model::{
        Event, EventDetails, EventLink, EventSchedule, EventTimeZone, Frequency, InputError,
        Recurrence, Reminder, Weekdays,
    },
    text::Language,
};

use super::{Dialog, EventAction, EventDraft};

pub(super) struct DetailsDraft {
    participants: widgets::EmailListDraft,
    attachments: String,
    frequency: Option<Frequency>,
    original_rule: Option<Recurrence>,
    original_timezone: EventTimeZone,
    end: widgets::RecurrenceEndDraft,
    weekdays: Option<Weekdays>,
    interval: NonZeroU16,
    reminders: Vec<Reminder>,
}

impl DetailsDraft {
    pub(super) fn from_event(event: &Event) -> Self {
        let rule = event.schedule.recurrence();
        Self {
            participants: widgets::EmailListDraft::from_emails(&event.details.participants),
            attachments: lines(event.details.attachments.iter().map(EventLink::as_str)),
            frequency: rule.map(Recurrence::frequency),
            original_rule: rule,
            original_timezone: event.schedule.timezone(),
            end: widgets::RecurrenceEndDraft::from_rule(rule),
            weekdays: rule.and_then(Recurrence::weekdays),
            interval: rule.map_or(NonZeroU16::MIN, Recurrence::interval),
            reminders: event.details.reminders.clone(),
        }
    }

    pub(super) fn take_details(
        &mut self,
        schedule: EventSchedule,
    ) -> Result<(EventSchedule, EventDetails), InputError> {
        self.participants.value()?;
        let attachments = self
            .attachments
            .lines()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.parse().map_err(|_| InputError::Attachment))
            .collect::<Result<Vec<_>, _>>()?;
        let recurrence = self
            .frequency
            .map(|frequency| {
                let end = self.end.parse()?;
                let until = end.until();
                let original_end = self
                    .original_rule
                    .filter(|rule| rule.until() == until && schedule.times().is_some());
                let weekdays = if frequency == Frequency::Weekly {
                    self.weekdays
                } else {
                    None
                };
                let until_time = original_end.and_then(Recurrence::until_time);
                let until_instant = original_end.and_then(|rule| {
                    let instant = rule.until_instant()?;
                    if self.original_timezone == schedule.timezone() {
                        Some(instant)
                    } else {
                        schedule.timezone().to_utc(until?.and_time(until_time?))
                    }
                });
                Recurrence::new(frequency, self.interval, until)?
                    .with_pattern(
                        end.count(),
                        weekdays,
                        self.original_rule
                            .map_or(chrono::Weekday::Mon, Recurrence::week_start),
                        until_time,
                    )?
                    .with_until_instant(until_instant)
            })
            .transpose()?;
        let schedule = schedule.with_recurrence(recurrence)?;
        Ok((
            schedule,
            EventDetails {
                participants: self.participants.take()?,
                attachments,
                reminders: std::mem::take(&mut self.reminders),
            },
        ))
    }

    fn recurrence_field(
        &mut self,
        ui: &mut egui::Ui,
        language: Language,
        default_date: impl FnOnce() -> chrono::NaiveDate,
    ) {
        widgets::FormRow::new(icons::REPEAT, language.text("Повторы", "Repeat")).show(ui, |ui| {
            let response = widgets::choice(
                ui,
                "event_repeat",
                &mut self.frequency,
                [
                    None,
                    Some(Frequency::Daily),
                    Some(Frequency::Weekly),
                    Some(Frequency::Monthly),
                    Some(Frequency::Yearly),
                ],
                |frequency| frequency_label(frequency, language),
            );
            widgets::hint(
                response,
                language.text(
                    "Окончание повторов: без ограничения, до даты или по количеству. Изменяется вся серия.",
                    "End repeats without a limit, on a date or after a count. Edits apply to the whole series.",
                ),
            );
            if self.frequency.is_some() {
                ui.horizontal_wrapped(|ui| {
                    ui.label(language.text("Каждые", "Every"));
                    let mut interval = self.interval.get();
                    ui.add(egui::DragValue::new(&mut interval).range(1..=u16::MAX));
                    self.interval = NonZeroU16::new(interval).unwrap_or(NonZeroU16::MIN);
                });
                ui.add(widgets::RecurrenceEndInput::new(
                    &mut self.end,
                    Dialog::Event.id().with("recurrence_end"),
                    language,
                    default_date,
                ));
                if self.frequency == Some(Frequency::Weekly) {
                    let mut enabled = self.weekdays.is_some();
                    if ui
                        .checkbox(&mut enabled, language.text("Дни недели", "Weekdays"))
                        .changed()
                    {
                        self.weekdays =
                            enabled.then(|| Weekdays::try_from(0x7f).expect("All weekdays"));
                    }
                    if let Some(days) = &mut self.weekdays {
                        ui.add(widgets::WeekdaySelector::new(days, language));
                    }
                }
            }
        });
    }

    pub(super) fn add_reminder(&mut self) {
        self.reminders.push(Reminder::DEFAULT);
    }

    pub(super) fn remove_reminder(&mut self, index: usize) {
        self.reminders.remove(index);
    }

    fn reminders_field(&mut self, ui: &mut egui::Ui, language: Language) -> EventAction {
        let mut action = EventAction::None;
        widgets::FormRow::new(icons::ALARM, language.text("Напоминания", "Reminders")).show(
            ui,
            |ui| {
                for (index, reminder) in self.reminders.iter_mut().enumerate() {
                    ui.push_id(index, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.add(widgets::ReminderInput::new(
                                reminder,
                                ui.make_persistent_id("reminder_value"),
                                language,
                            ));
                            if ui
                                .button("×")
                                .on_hover_text(
                                    language.text("Удалить напоминание", "Remove reminder"),
                                )
                                .clicked()
                            {
                                action = EventAction::RemoveReminder(index);
                            }
                        });
                    });
                }
                if ui
                    .push_id("add_reminder", |ui| {
                        widgets::hint(
                            ui.button(language.text("+ Напоминание", "+ Reminder")),
                            language.text(
                                "Время до начала события; 0 — в момент начала.",
                                "Time before the event; 0 means at the start.",
                            ),
                        )
                    })
                    .inner
                    .clicked()
                {
                    action = EventAction::AddReminder;
                }
            },
        );
        action
    }
}

impl Default for DetailsDraft {
    fn default() -> Self {
        Self {
            participants: widgets::EmailListDraft::default(),
            attachments: String::new(),
            frequency: None,
            original_rule: None,
            original_timezone: EventTimeZone::Floating,
            end: widgets::RecurrenceEndDraft::Never,
            weekdays: None,
            interval: NonZeroU16::MIN,
            reminders: Vec::new(),
        }
    }
}

impl EventDraft {
    pub(super) fn detail_fields(&mut self, ui: &mut egui::Ui, language: Language) -> EventAction {
        let default_date = || {
            widgets::parse_date(&self.end)
                .or_else(|_| widgets::parse_date(&self.start))
                .unwrap_or(chrono::NaiveDate::MIN)
        };
        self.details.recurrence_field(ui, language, default_date);
        widgets::FormRow::new(icons::USERS, language.text("Участники", "Participants")).show(
            ui,
            |ui| {
                ui.add(widgets::EmailListInput::new(
                    &mut self.details.participants,
                    Dialog::Event.id().with("participants"),
                    language,
                ));
            },
        );
        widgets::FormRow::new(icons::PAPERCLIP, language.text("Вложения", "Attachments")).show(
            ui,
            |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.details.attachments)
                        .id(Dialog::Event.id().with("attachments"))
                        .frame(widgets::form_text_frame(ui))
                        .desired_width(f32::INFINITY)
                        .desired_rows(1)
                        .hint_text(language.text(
                            "Ссылки на файлы — по одной на строку",
                            "File links, one per line",
                        ))
                        .with_context_menu(language),
                );
            },
        );
        self.details.reminders_field(ui, language)
    }
}

fn lines<'a>(values: impl Iterator<Item = &'a str>) -> String {
    let mut text = String::new();
    for value in values {
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(value);
    }
    text
}

const fn frequency_label(frequency: Option<Frequency>, language: Language) -> &'static str {
    match frequency {
        None => language.text("Не повторять", "Does not repeat"),
        Some(Frequency::Daily) => language.text("Ежедневно", "Daily"),
        Some(Frequency::Weekly) => language.text("Еженедельно", "Weekly"),
        Some(Frequency::Monthly) => language.text("Ежемесячно", "Monthly"),
        Some(Frequency::Yearly) => language.text("Ежегодно", "Yearly"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{CategoryId, DateRange, Document, EventId};

    #[test]
    fn saving_moves_reminders_only_after_all_fields_are_parsed() {
        let document = Document::default();
        for error in [
            InputError::EmptyTitle,
            InputError::EventLink,
            InputError::Participant,
            InputError::Attachment,
            InputError::RecurrenceUntil,
        ] {
            let mut draft = EventDraft::new(document.year.range(), CategoryId(1));
            draft.title = "Meeting".into();
            draft.notes = "Keep the notes".into();
            draft.location = "Keep the location".into();
            draft.link = "https://example.com/call".into();
            draft.details.participants = "ok@example.com".into();
            draft.details.reminders = vec![Reminder::DEFAULT];
            match error {
                InputError::EmptyTitle => draft.title.clear(),
                InputError::EventLink => draft.link = "invalid-link".into(),
                InputError::Participant => draft.details.participants = "bad-email".into(),
                InputError::Attachment => draft.details.attachments = "invalid-link".into(),
                InputError::RecurrenceUntil => {
                    draft.details.frequency = Some(Frequency::Daily);
                    draft.details.end = widgets::RecurrenceEndDraft::Until("invalid-date".into());
                }
                _ => unreachable!(),
            }
            let link = draft.link.as_str().to_owned();
            let participants = draft.details.participants.as_str().to_owned();
            assert!(
                matches!(draft.take_event(EventId(1), &document), Err(actual) if actual == error)
            );
            assert_eq!(draft.link.as_str(), link);
            assert_eq!(draft.details.participants.as_str(), participants);
            assert_eq!(draft.details.reminders, [Reminder::DEFAULT]);
            assert_eq!(draft.notes, "Keep the notes");
            assert_eq!(draft.location, "Keep the location");
            draft.title = "Meeting".into();
            draft.link.clear();
            draft.details.participants.clear();
            draft.details.attachments.clear();
            draft.details.end = widgets::RecurrenceEndDraft::Never;
            let event = draft.take_event(EventId(1), &document).unwrap();
            assert_eq!(event.details.reminders, [Reminder::DEFAULT]);
            assert_eq!(draft.details.reminders, [] as [Reminder; 0]);
        }
    }

    #[test]
    fn new_details_round_trip_and_invalid_inputs_preserve_the_draft() {
        let mut document = Document::default();
        let date = "2026-10-02".parse().unwrap();
        let mut draft = EventDraft::new(DateRange::between(date, date), CategoryId(1));
        draft.title = "Project".into();
        draft.notes = "Details".into();
        draft.details.participants = "a@example.com; b@example.com".into();
        draft.details.attachments = "https://example.com/a.pdf\nhttps://example.com/b.pdf".into();
        draft.details.frequency = Some(Frequency::Weekly);
        draft.details.interval = NonZeroU16::new(2).unwrap();
        draft.details.end = widgets::RecurrenceEndDraft::Until("2027-10-02".into());
        draft.details.reminders = [0, 15, 1440]
            .map(|minutes| Reminder::try_from(minutes).unwrap())
            .into();
        document
            .events
            .push(draft.take_event(EventId(1), &document).unwrap());
        let restored = Document::parse(&serde_json::to_string(&document).unwrap()).unwrap();
        let mut editor = EventDraft::from_event(&restored.events[0]);
        assert_eq!(
            editor.details.participants.as_str(),
            "a@example.com\nb@example.com"
        );
        let event = editor.take_event(EventId(1), &restored).unwrap();
        assert_eq!(
            serde_json::to_value(&event).unwrap(),
            serde_json::to_value(&restored.events[0]).unwrap()
        );
        editor.notes = "Keep these notes".into();
        editor.details.end = widgets::RecurrenceEndDraft::Until("2026-10-01".into());
        assert!(matches!(
            editor.take_event(EventId(1), &restored),
            Err(InputError::RecurrenceUntil)
        ));
        editor.details.end = widgets::RecurrenceEndDraft::Never;
        editor.details.attachments = "invalid-link".into();
        assert!(matches!(
            editor.take_event(EventId(1), &restored),
            Err(InputError::Attachment)
        ));
        editor.details.attachments.clear();
        editor.details.participants = "bad-email".into();
        assert!(matches!(
            editor.take_event(EventId(1), &restored),
            Err(InputError::Participant)
        ));
        assert_eq!(editor.notes, "Keep these notes");
        assert_eq!(editor.details.participants.as_str(), "bad-email");
        let mut value = serde_json::to_value(event).unwrap();
        value["recurrence"]["until"] = serde_json::json!("2026-10-01");
        assert!(serde_json::from_value::<Event>(value).is_err());
    }
}
