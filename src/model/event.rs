use chrono::{NaiveDateTime, NaiveTime};
use serde::{Deserialize, Serialize};

use super::{CategoryId, DateRange, EventId, InputError, Title};

pub use details::{Email, EventDetails, Reminder};
pub use identity::{EventIdentity, EventUid, OccurrenceStart};
pub use recurrence::{Frequency, Recurrence, Weekdays};
pub use timezone::EventTimeZone;

mod details;
mod identity;
mod recurrence;
mod timezone;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventStatus {
    Tentative,
    Confirmed,
    Cancelled,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Availability {
    #[default]
    Busy,
    Free,
}

#[derive(Serialize, Deserialize)]
pub struct Event {
    pub id: EventId,
    #[serde(default)]
    pub identity: EventIdentity,
    pub title: Title,
    #[serde(flatten)]
    pub schedule: EventSchedule,
    pub category: CategoryId,
    pub notes: String,
    #[serde(default)]
    pub location: String,
    #[serde(default)]
    pub link: Option<EventLink>,
    #[serde(default)]
    pub status: Option<EventStatus>,
    #[serde(default)]
    pub availability: Availability,
    #[serde(flatten)]
    pub details: EventDetails,
}

impl Event {
    pub fn occurrences(&self, query: DateRange) -> impl Iterator<Item = EventSchedule> + '_ {
        self.schedule.occurrences(query).filter(|schedule| {
            self.identity.exclusions.is_empty()
                || !OccurrenceStart::at(*schedule, schedule.start())
                    .is_some_and(|start| self.identity.exclusions.contains(start))
        })
    }
}

/// Даты окончания включительны для событий на весь день; время окончания исключает конечный момент.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "ScheduleInput", into = "ScheduleInput")]
pub struct EventSchedule {
    dates: DateRange,
    kind: ScheduleKind,
    recurrence: Option<Recurrence>,
    timezone: EventTimeZone,
}

/// Занятые даты нужны только событиям со временем; для событий на весь день они совпадают с датами расписания.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScheduleKind {
    AllDay,
    Timed {
        times: EventTimes,
        occupied: DateRange,
    },
}

impl EventSchedule {
    #[must_use]
    pub const fn all_day(dates: DateRange) -> Self {
        Self {
            dates,
            kind: ScheduleKind::AllDay,
            recurrence: None,
            timezone: EventTimeZone::Floating,
        }
    }

    /// # Errors
    /// Возвращает ошибку, если окончание предшествует началу.
    pub fn timed(dates: DateRange, times: EventTimes) -> Result<Self, InputError> {
        if dates.start().and_time(times.start) > dates.end().and_time(times.end) {
            return Err(InputError::ReversedTime);
        }
        // Проекция вычисляется один раз, чтобы индекс не проверял границу повторно.
        let occupied = if times.end == NaiveTime::MIN && dates.end() > dates.start() {
            let end = dates.end().pred_opt().ok_or(InputError::ReversedTime)?;
            DateRange::new(dates.start(), end)?
        } else {
            dates
        };
        Ok(Self {
            dates,
            kind: ScheduleKind::Timed { times, occupied },
            recurrence: None,
            timezone: EventTimeZone::Floating,
        })
    }

    /// # Errors
    /// Возвращает ошибку, если правило повторов несовместимо с расписанием.
    pub fn with_recurrence(mut self, recurrence: Option<Recurrence>) -> Result<Self, InputError> {
        if let Some(rule) = recurrence {
            rule.validate_for(self)?;
        }
        self.recurrence = recurrence;
        Ok(self)
    }

    /// # Errors
    /// Возвращает ошибку для зоны события на весь день, некорректного порядка времени или несовместимых повторов.
    pub fn with_timezone(mut self, timezone: EventTimeZone) -> Result<Self, InputError> {
        if self.times().is_none() && timezone != EventTimeZone::Floating {
            return Err(InputError::StartTime);
        }
        self.timezone = timezone;
        if let Some(times) = self.times()
            && timezone
                .to_utc(self.start())
                .zip(timezone.to_utc(self.dates.end().and_time(times.end)))
                .is_some_and(|(start, end)| start > end)
        {
            return Err(InputError::ReversedTime);
        }
        if let Some(rule) = self.recurrence {
            rule.validate_for(self)?;
        }
        Ok(self)
    }

    #[must_use]
    pub fn start(self) -> NaiveDateTime {
        self.dates
            .start()
            .and_time(self.times().map_or(NaiveTime::MIN, |times| times.start))
    }

    #[must_use]
    pub const fn timezone(self) -> EventTimeZone {
        self.timezone
    }

    #[must_use]
    pub const fn recurrence(self) -> Option<Recurrence> {
        self.recurrence
    }

    #[must_use]
    pub const fn dates(self) -> DateRange {
        self.dates
    }

    #[must_use]
    pub const fn times(self) -> Option<EventTimes> {
        match self.kind {
            ScheduleKind::AllDay => None,
            ScheduleKind::Timed { times, .. } => Some(times),
        }
    }

    /// Полночь на правой границе не занимает день окончания в годовой сетке.
    #[must_use]
    pub const fn occupied_dates(self) -> DateRange {
        match self.kind {
            ScheduleKind::AllDay => self.dates,
            ScheduleKind::Timed { occupied, .. } => occupied,
        }
    }
}

impl TryFrom<ScheduleInput> for EventSchedule {
    type Error = InputError;

    fn try_from(value: ScheduleInput) -> Result<Self, Self::Error> {
        let schedule = match value.times {
            Some(times) => Self::timed(value.range, times)?,
            None => Self::all_day(value.range),
        };
        schedule
            .with_timezone(value.timezone)?
            .with_recurrence(value.recurrence)
    }
}

/// Местное время границ; их порядок проверяется вместе с датами в `EventSchedule::timed`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventTimes {
    pub start: NaiveTime,
    pub end: NaiveTime,
}

#[derive(Serialize, Deserialize)]
struct ScheduleInput {
    #[serde(default)]
    timezone: EventTimeZone,
    range: DateRange,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    times: Option<EventTimes>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    recurrence: Option<Recurrence>,
}

impl From<EventSchedule> for ScheduleInput {
    fn from(value: EventSchedule) -> Self {
        Self {
            range: value.dates,
            timezone: value.timezone,
            times: value.times(),
            recurrence: value.recurrence,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct EventLink(url::Url);

impl EventLink {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl TryFrom<String> for EventLink {
    type Error = InputError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl std::str::FromStr for EventLink {
    type Err = InputError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let value = value.trim();
        let Some((scheme, _)) = value.split_once("://") else {
            return Err(InputError::EventLink);
        };
        if !(scheme.eq_ignore_ascii_case("https") || scheme.eq_ignore_ascii_case("http"))
            || value.chars().any(char::is_whitespace)
        {
            return Err(InputError::EventLink);
        }
        let url = url::Url::parse(value).map_err(|_| InputError::EventLink)?;
        if url.host_str().is_none() {
            return Err(InputError::EventLink);
        }
        Ok(Self(url))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr as _;

    #[test]
    fn timed_schedules_validate_order_and_exclude_midnight_from_occupied_days() {
        let same_day =
            DateRange::between("2026-10-02".parse().unwrap(), "2026-10-02".parse().unwrap());
        let overnight = DateRange::between(same_day.start(), "2026-10-03".parse().unwrap());
        let times = EventTimes {
            start: "23:00".parse().unwrap(),
            end: NaiveTime::MIN,
        };
        assert!(EventSchedule::timed(same_day, times).is_err());
        let schedule = EventSchedule::timed(overnight, times).unwrap();
        assert_eq!(schedule.occupied_dates(), same_day);
        assert_eq!(schedule.dates(), overnight);
        assert_eq!(
            serde_json::from_value::<EventSchedule>(serde_json::to_value(schedule).unwrap())
                .unwrap(),
            schedule
        );
        assert!(
            serde_json::from_value::<EventSchedule>(
                serde_json::json!({"range": same_day, "times": times})
            )
            .is_err()
        );
        assert!(
            EventSchedule::timed(
                same_day,
                EventTimes {
                    start: times.start,
                    end: times.start
                }
            )
            .is_ok()
        );
    }

    #[test]
    fn timezone_changes_preserve_schedule_invariants() {
        let date = "2026-03-29".parse().unwrap();
        let dates = DateRange::between(date, date);
        let zone = EventTimeZone::Named(chrono_tz::Europe::Berlin);
        assert!(EventSchedule::all_day(dates).with_timezone(zone).is_err());
        let gap = EventSchedule::timed(
            dates,
            EventTimes {
                start: "02:30".parse().unwrap(),
                end: "03:00".parse().unwrap(),
            },
        )
        .unwrap();
        assert!(matches!(
            gap.with_timezone(zone),
            Err(InputError::ReversedTime)
        ));
        let rule = Recurrence::new(Frequency::Daily, std::num::NonZeroU16::MIN, Some(date))
            .unwrap()
            .with_pattern(
                None,
                None,
                chrono::Weekday::Mon,
                Some("09:00".parse().unwrap()),
            )
            .unwrap()
            .with_until_instant(Some("2026-03-29T07:00:00Z".parse().unwrap()))
            .unwrap();
        let schedule = EventSchedule::timed(
            dates,
            EventTimes {
                start: "08:00".parse().unwrap(),
                end: "09:00".parse().unwrap(),
            },
        )
        .unwrap()
        .with_timezone(zone)
        .unwrap()
        .with_recurrence(Some(rule))
        .unwrap();
        assert!(schedule.with_timezone(EventTimeZone::Utc).is_err());
        assert!(schedule.with_timezone(EventTimeZone::Floating).is_err());
    }

    #[test]
    fn links_are_parsed_on_all_input_paths() {
        for value in [
            " https://example.com/meeting ",
            "http://localhost:8080/",
            "HTTPS://EXAMPLE.COM/meeting",
        ] {
            let link = EventLink::from_str(value).unwrap();
            let restored: EventLink =
                serde_json::from_value(serde_json::to_value(&link).unwrap()).unwrap();
            assert_eq!(restored, link);
        }
        for value in [
            "example.com",
            "https://",
            "javascript:alert(1)",
            "https://example.com/a b",
        ] {
            assert!(EventLink::from_str(value).is_err());
            assert!(serde_json::from_value::<EventLink>(serde_json::json!(value)).is_err());
        }
    }

    #[test]
    fn old_events_load_as_all_day_and_new_fields_round_trip() {
        let old = serde_json::json!({"id": 1, "title": "Meeting", "range": {"start": "2026-10-02", "end": "2026-10-02"}, "category": 1, "notes": "Details"});
        let mut event: Event = serde_json::from_value(old).unwrap();
        assert_eq!(event.schedule.times(), None);
        assert_eq!(event.availability, Availability::Busy);
        event.location = "Office".into();
        event.link = Some(EventLink::from_str("https://example.com/").unwrap());
        event.status = Some(EventStatus::Tentative);
        event.availability = Availability::Free;
        let restored: Event =
            serde_json::from_value(serde_json::to_value(&event).unwrap()).unwrap();
        assert_eq!(restored.location, event.location);
        assert_eq!(restored.link, event.link);
        assert_eq!(restored.status, event.status);
        assert_eq!(restored.availability, event.availability);
    }
}
