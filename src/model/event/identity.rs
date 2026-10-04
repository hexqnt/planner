use super::{EventSchedule, EventTimeZone};
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct EventUid(String);

impl EventUid {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for EventUid {
    fn default() -> Self {
        Self(format!("{}@planner", uuid::Uuid::new_v4()))
    }
}

impl TryFrom<String> for EventUid {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.trim().is_empty() || value.contains(['\r', '\n', '\0']) {
            return Err("Invalid event UID");
        }
        Ok(Self(value))
    }
}

#[derive(Default, Serialize, Deserialize)]
pub struct EventIdentity {
    #[serde(default)]
    pub uid: EventUid,
    #[serde(default)]
    pub exclusions: Exclusions,
}

/// Тип начала исключения сохраняет различие между датой, настенными часами и точным моментом UTC.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum OccurrenceStart {
    Date(NaiveDate),
    Local(NaiveDateTime),
    Instant(DateTime<Utc>),
}

impl OccurrenceStart {
    pub fn at(schedule: EventSchedule, time: NaiveDateTime) -> Option<Self> {
        if schedule.times().is_none() {
            return Some(Self::Date(time.date()));
        }
        match schedule.timezone() {
            EventTimeZone::Floating => Some(Self::Local(time)),
            zone => zone.to_utc(time).map(Self::Instant),
        }
    }

    pub fn wall_time(self, zone: EventTimeZone) -> NaiveDateTime {
        match self {
            Self::Date(date) => date.and_time(NaiveTime::MIN),
            Self::Local(time) => time,
            Self::Instant(time) => zone.wall_time(time),
        }
    }

    pub const fn matches_schedule_type(self, schedule: EventSchedule) -> bool {
        matches!(
            (self, schedule.times().is_some(), schedule.timezone()),
            (Self::Date(_), false, _)
                | (Self::Local(_), true, EventTimeZone::Floating)
                | (
                    Self::Instant(_),
                    true,
                    EventTimeZone::Utc | EventTimeZone::Named(_)
                )
        )
    }
}

/// Сортировка выполняется на границе ввода; поиск исключений не выделяет память.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "Vec<OccurrenceStart>")]
pub struct Exclusions(Vec<OccurrenceStart>);

impl From<Vec<OccurrenceStart>> for Exclusions {
    fn from(mut values: Vec<OccurrenceStart>) -> Self {
        values.sort_unstable();
        values.dedup();
        Self(values)
    }
}

impl Exclusions {
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn map_in_place(&mut self, mut transform: impl FnMut(OccurrenceStart) -> OccurrenceStart) {
        for start in &mut self.0 {
            *start = transform(*start);
        }
        self.0.sort_unstable();
        self.0.dedup();
    }

    pub fn contains(&self, start: OccurrenceStart) -> bool {
        self.0.binary_search(&start).is_ok()
    }

    pub fn iter(&self) -> impl Iterator<Item = OccurrenceStart> + '_ {
        self.0.iter().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        model::EventTimes,
        test_support::{date, range},
    };

    #[test]
    fn occurrence_starts_and_exclusion_types_follow_the_schedule_kind_and_timezone() {
        let dates = range("2026-10-03", "2026-10-03");
        let local = date("2026-10-03").and_hms_opt(9, 0, 0).unwrap();
        let all_day = EventSchedule::all_day(dates);
        let day = OccurrenceStart::Date(dates.start());
        let wall = OccurrenceStart::Local(local);
        let instant = OccurrenceStart::Instant(local.and_utc());
        assert_eq!(OccurrenceStart::at(all_day, all_day.start()), Some(day));
        assert_eq!(
            day.wall_time(EventTimeZone::Utc),
            dates.start().and_time(NaiveTime::MIN)
        );
        let timed = EventSchedule::timed(
            dates,
            EventTimes {
                start: local.time(),
                end: "10:00".parse().unwrap(),
            },
        )
        .unwrap();
        for (zone, expected) in [
            (EventTimeZone::Floating, wall),
            (EventTimeZone::Utc, instant),
            (
                EventTimeZone::Named(chrono_tz::Europe::Moscow),
                OccurrenceStart::Instant("2026-10-03T06:00:00Z".parse().unwrap()),
            ),
        ] {
            let schedule = timed.with_timezone(zone).unwrap();
            assert_eq!(OccurrenceStart::at(schedule, local), Some(expected));
            assert_eq!(expected.wall_time(zone), local);
            for candidate in [day, wall, instant] {
                assert_eq!(candidate.matches_schedule_type(all_day), candidate == day);
                let matches = if zone == EventTimeZone::Floating {
                    candidate == wall
                } else {
                    candidate == instant
                };
                assert_eq!(
                    candidate.matches_schedule_type(schedule),
                    matches,
                    "{candidate:?}, {zone:?}"
                );
            }
        }
    }

    #[test]
    fn uids_preserve_text_and_reject_empty_or_control_characters() {
        for value in ["event@example.org", " UID with spaces ", "событие"] {
            let uid = EventUid::try_from(value.to_owned()).unwrap();
            assert_eq!(uid.as_str(), value);
            let restored: EventUid = serde_json::from_value(serde_json::json!(value)).unwrap();
            assert_eq!(restored, uid);
            assert_eq!(serde_json::to_value(uid).unwrap(), value);
        }
        for value in ["", " \t ", "event\rname", "event\nname", "event\0name"] {
            assert_eq!(
                EventUid::try_from(value.to_owned()),
                Err("Invalid event UID"),
                "{value:?}"
            );
            assert!(
                serde_json::from_value::<EventUid>(serde_json::json!(value)).is_err(),
                "{value:?}"
            );
        }
    }

    #[test]
    fn exclusions_are_sorted_and_deduplicated_on_both_input_paths() {
        let first = OccurrenceStart::Date(date("2026-10-03"));
        let last = OccurrenceStart::Date(date("2026-10-05"));
        let values = vec![last, first, last, first];
        for exclusions in [
            Exclusions::from(values.clone()),
            serde_json::from_value::<Exclusions>(serde_json::to_value(values).unwrap()).unwrap(),
        ] {
            assert!(!exclusions.is_empty());
            assert_eq!(exclusions.iter().collect::<Vec<_>>(), [first, last]);
            assert!(exclusions.contains(first));
            assert!(exclusions.contains(last));
            assert!(!exclusions.contains(OccurrenceStart::Date(date("2026-10-04"))));
        }
        let empty = Exclusions::default();
        assert!(empty.is_empty());
        assert!(!empty.contains(first));
    }

    #[test]
    fn transforming_exclusions_restores_order_and_collapses_collisions() {
        let first = OccurrenceStart::Date(date("2026-10-03"));
        let middle = OccurrenceStart::Date(date("2026-10-04"));
        let last = OccurrenceStart::Date(date("2026-10-05"));
        let mut exclusions = Exclusions::from(vec![first, middle, last]);
        exclusions.map_in_place(|start| if start == last { first } else { last });
        assert_eq!(exclusions.iter().collect::<Vec<_>>(), [first, last]);
        assert!(exclusions.contains(first));
        assert!(exclusions.contains(last));
        assert!(!exclusions.contains(middle));
    }
}
