use chrono::{DateTime, Duration, LocalResult, NaiveDateTime, Offset as _, TimeZone as _, Utc};
use serde::{Deserialize, Serialize};

/// Расписание хранит настенные часы исходной зоны, поэтому повторения не сдвигаются при переходе на летнее время.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventTimeZone {
    #[default]
    Floating,
    Utc,
    Named(chrono_tz::Tz),
}

impl EventTimeZone {
    pub fn to_utc(self, time: NaiveDateTime) -> Option<DateTime<Utc>> {
        match self {
            Self::Floating => None,
            Self::Utc => Some(time.and_utc()),
            Self::Named(zone) => {
                match zone.from_local_datetime(&time) {
                    LocalResult::Single(time) | LocalResult::Ambiguous(time, _) => {
                        Some(time.to_utc())
                    }
                    // RFC 5545: для пропущенного часа применяется смещение перед переходом.
                    LocalResult::None => (1..=1440).find_map(|minutes| {
                        let previous = time.checked_sub_signed(Duration::minutes(minutes))?;
                        let offset = zone
                            .from_local_datetime(&previous)
                            .earliest()?
                            .offset()
                            .fix();
                        Some(offset.from_local_datetime(&time).single()?.to_utc())
                    }),
                }
            }
        }
    }

    pub fn wall_time(self, time: DateTime<Utc>) -> NaiveDateTime {
        match self {
            Self::Named(zone) => time.with_timezone(&zone).naive_local(),
            Self::Floating | Self::Utc => time.naive_utc(),
        }
    }

    pub fn exists(self, time: NaiveDateTime) -> bool {
        match self {
            Self::Named(zone) => !matches!(zone.from_local_datetime(&time), LocalResult::None),
            Self::Floating | Self::Utc => true,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Floating => "Floating",
            Self::Utc => "UTC",
            Self::Named(zone) => zone.name(),
        }
    }
}
