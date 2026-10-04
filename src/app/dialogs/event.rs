use crate::app::widgets::{LinkDraft, format_time, parse_date, parse_time};
use chrono::NaiveTime;

use crate::app::{shortcuts::Shortcut, ui_config::dialog};
use crate::model::{
    Availability, CategoryId, DateRange, DisplayTimeZone, Document, Event, EventId, EventIdentity,
    EventSchedule, EventStatus, EventTimeZone, EventTimes, InputError, OccurrenceStart, Title,
};

use super::{Dialog, Planner};
use details::DetailsDraft;

mod details;
mod fields;

#[derive(Default)]
enum EventAction {
    #[default]
    None,
    Save,
    Cancel,
    RequestDeletion,
    AddReminder,
    RemoveReminder(usize),
}

/// Подтверждение удаления возможно только для существующего события и всегда хранит его ID.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EventEditorState {
    Creating,
    Editing(EventId),
    ConfirmingDeletion(EventId),
}

impl EventEditorState {
    const fn event_id(self) -> Option<EventId> {
        match self {
            Self::Creating => None,
            Self::Editing(id) | Self::ConfirmingDeletion(id) => Some(id),
        }
    }
}

pub(in crate::app) struct EventDraft {
    state: EventEditorState,
    identity: EventIdentity,
    timezone: EventTimeZone,
    original_timezone: EventTimeZone,
    timezone_picker: crate::app::layout::timezone::TimeZonePicker,
    original_start_time: NaiveTime,
    title: String,
    focus_title: bool,
    start: String,
    end: String,
    all_day: bool,
    start_time: String,
    end_time: String,
    location: String,
    link: LinkDraft,
    status: Option<EventStatus>,
    availability: Availability,
    category: CategoryId,
    notes: String,
    details: DetailsDraft,
    error: Option<InputError>,
    pending_action: EventAction,
}

impl EventDraft {
    #[cfg(test)]
    pub(in crate::app) const fn category(&self) -> CategoryId {
        self.category
    }
    pub(in crate::app) fn set_default_timezone(&mut self, zone: DisplayTimeZone) {
        self.timezone = match zone {
            DisplayTimeZone::Utc => EventTimeZone::Utc,
            DisplayTimeZone::Named(zone) => EventTimeZone::Named(zone),
            DisplayTimeZone::System => iana_time_zone::get_timezone()
                .ok()
                .and_then(|name| name.parse().ok())
                .map_or(EventTimeZone::Floating, EventTimeZone::Named),
        };
    }

    pub(in crate::app) fn set_title(&mut self, title: &str) {
        title.clone_into(&mut self.title);
    }
    #[cfg(feature = "testing")]
    pub(in crate::app) const fn error(&self) -> Option<InputError> {
        self.error
    }

    pub(in crate::app) fn new(range: DateRange, category: CategoryId) -> Self {
        Self::with_schedule(
            EventSchedule::all_day(range),
            category,
            EventIdentity::default(),
        )
    }

    fn with_schedule(
        schedule: EventSchedule,
        category: CategoryId,
        identity: EventIdentity,
    ) -> Self {
        let range = schedule.dates();
        let (all_day, start_time, end_time) = schedule.times().map_or_else(
            || (true, "09:00".into(), "10:00".into()),
            |times| (false, format_time(times.start), format_time(times.end)),
        );
        Self {
            state: EventEditorState::Creating,
            identity,
            timezone: schedule.timezone(),
            original_timezone: schedule.timezone(),
            timezone_picker: crate::app::layout::timezone::TimeZonePicker::default(),
            original_start_time: schedule.times().map_or(NaiveTime::MIN, |times| times.start),
            title: String::new(),
            focus_title: true,
            start: range.start().to_string(),
            end: range.end().to_string(),
            all_day,
            start_time,
            end_time,
            location: String::new(),
            link: LinkDraft::default(),
            status: None,
            availability: Availability::Busy,
            category,
            notes: String::new(),
            details: DetailsDraft::default(),
            error: None,
            pending_action: EventAction::None,
        }
    }

    pub(in crate::app) fn from_event(event: &Event) -> Self {
        Self {
            state: EventEditorState::Editing(event.id),
            title: event.title.get().into(),
            focus_title: false,
            notes: event.notes.clone(),
            details: DetailsDraft::from_event(event),
            location: event.location.clone(),
            link: LinkDraft::from_link(event.link.as_ref()),
            status: event.status,
            availability: event.availability,
            ..Self::with_schedule(
                event.schedule,
                event.category,
                EventIdentity {
                    uid: event.identity.uid.clone(),
                    exclusions: event.identity.exclusions.clone(),
                },
            )
        }
    }

    fn take_event(&mut self, id: EventId, document: &Document) -> Result<Event, InputError> {
        if document.category(self.category).is_none() {
            return Err(InputError::UnknownCategory);
        }
        let schedule = self.parse_schedule()?;
        self.link.value()?;
        let title = Title::try_from(self.title.as_str())?;
        let (schedule, details) =
            self.details
                .take_details(schedule.with_timezone(if self.all_day {
                    EventTimeZone::Floating
                } else {
                    self.timezone
                })?)?;
        let link = self.link.take()?;
        let mut identity = std::mem::take(&mut self.identity);
        let start_time = schedule.times().map_or(NaiveTime::MIN, |times| times.start);
        if start_time != self.original_start_time
            || schedule.times().is_none()
            || schedule.timezone() != self.original_timezone
        {
            // При смене времени всей серии исключённые дни сохраняются; целому дню соответствует полночь.
            identity.exclusions.map_in_place(|time| {
                let local = time.wall_time(self.original_timezone);
                if schedule.times().is_none() || local.time() == self.original_start_time {
                    OccurrenceStart::at(schedule, local.date().and_time(start_time)).unwrap_or(time)
                } else if schedule.timezone() != self.original_timezone {
                    OccurrenceStart::at(schedule, local).unwrap_or(time)
                } else {
                    time
                }
            });
        }
        Ok(Event {
            id,
            identity,
            title,
            schedule,
            category: self.category,
            notes: std::mem::take(&mut self.notes),
            location: std::mem::take(&mut self.location),
            link,
            status: self.status,
            availability: self.availability,
            details,
        })
    }

    fn parse_schedule(&self) -> Result<EventSchedule, InputError> {
        let start = parse_date(&self.start).map_err(|_| InputError::StartDate)?;
        let end = parse_date(&self.end).map_err(|_| InputError::EndDate)?;
        let range = DateRange::new(start, end)?;
        if self.all_day {
            Ok(EventSchedule::all_day(range))
        } else {
            EventSchedule::timed(
                range,
                EventTimes {
                    start: parse_time(&self.start_time).map_err(|_| InputError::StartTime)?,
                    end: parse_time(&self.end_time).map_err(|_| InputError::EndTime)?,
                },
            )
        }
    }
}

impl Planner {
    pub(super) fn event_dialog(&mut self, ctx: &egui::Context) {
        let language = self.language();
        let Some(draft) = &mut self.event_draft else {
            return;
        };
        let pending_action = std::mem::take(&mut draft.pending_action);
        let mut action = if ctx
            .top_layer_id()
            .is_some_and(|layer| layer.id == Dialog::Event.id())
            && !self.shortcuts.popup_open(ctx)
            && self.shortcuts.consume(Shortcut::SaveEvent, ctx)
        {
            EventAction::Save
        } else {
            pending_action
        };
        let previous_category = draft.category;
        Dialog::Event
            .window(ctx, language.text("Событие", "Event"))
            .min_size(dialog::FORM_WINDOW_MIN_SIZE)
            .default_width(
                dialog::FORM_WINDOW_DEFAULT_SIZE
                    .x
                    .min(ctx.content_rect().width() - dialog::FORM_SCREEN_RESERVE.x),
            )
            .default_height(
                dialog::FORM_WINDOW_DEFAULT_SIZE
                    .y
                    .min(ctx.content_rect().height() - dialog::FORM_SCREEN_RESERVE.y),
            )
            .resizable(true)
            .show(ctx, |ui| {
                if draft.header(ui, language) {
                    action = EventAction::Cancel;
                }
                ui.add_space(dialog::HEADER_GAP);
                let footer_action = draft.body(ui, &self.document);
                if !matches!(footer_action, EventAction::None) {
                    action = footer_action;
                }
            });
        if draft.category != previous_category {
            self.document.last_event_category = Some(draft.category);
            self.persistence.mark_changed();
        }
        if ctx.will_discard() {
            // egui может не повторить клик в следующем проходе; действие сохраняется до принятой отрисовки.
            draft.pending_action = action;
            return;
        }
        match action {
            EventAction::AddReminder => {
                draft.details.add_reminder();
                ctx.request_repaint();
                return;
            }
            EventAction::RemoveReminder(index) => {
                draft.details.remove_reminder(index);
                ctx.request_repaint();
                return;
            }
            EventAction::None => return,
            EventAction::Cancel => {
                self.event_draft = None;
                return;
            }
            EventAction::Save => {
                let id = draft
                    .state
                    .event_id()
                    .unwrap_or_else(|| self.document.next_event_id());
                let event = match draft.take_event(id, &self.document) {
                    Ok(event) => event,
                    Err(error) => {
                        draft.error = Some(error);
                        return;
                    }
                };
                self.remember_event_category(event.category);
                self.save_event(event);
            }
            EventAction::RequestDeletion => match draft.state {
                EventEditorState::Creating => return,
                EventEditorState::Editing(id) => {
                    draft.state = EventEditorState::ConfirmingDeletion(id);
                    ctx.request_repaint();
                    return;
                }
                EventEditorState::ConfirmingDeletion(id) => {
                    self.document.events.retain(|event| event.id != id);
                }
            },
        }
        self.mark_changed();
        self.event_draft = None;
        ctx.request_repaint();
    }

    fn save_event(&mut self, event: Event) {
        if let Some(range) = event
            .schedule
            .displayed_dates(self.document.display_timezone)
        {
            self.selection.set_range(range);
        } else {
            self.selection.clear();
        }
        if let Some(existing) = self
            .document
            .events
            .iter_mut()
            .find(|existing| existing.id == event.id)
        {
            *existing = event;
        } else {
            self.document.events.push(event);
        }
    }
}
#[cfg(test)]
mod tests;
