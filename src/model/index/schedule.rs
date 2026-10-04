use std::fmt;

use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime};

use crate::model::{DateRange, DisplayTimeZone, EventSchedule, EventTimeZone, Year};

#[cfg(test)]
mod tests;

/// Настенные часы проекции могут идти назад при переводе часов, хотя абсолютный интервал корректен.
#[derive(Clone, Copy)]
pub enum DisplayedSchedule {
    AllDay(DateRange),
    Timed {
        start: NaiveDateTime,
        end: NaiveDateTime,
        occupied: DateRange,
    },
}

impl DisplayedSchedule {
    pub fn project(schedule: EventSchedule, zone: DisplayTimeZone) -> Option<Self> {
        let dates = schedule.dates();
        let Some(times) = schedule.times() else {
            return Some(Self::AllDay(dates));
        };
        let source = schedule.timezone();
        let start = schedule.start();
        let end = dates.end().and_time(times.end);
        let unchanged = matches!(
            (source, zone),
            (EventTimeZone::Floating, _) | (EventTimeZone::Utc, DisplayTimeZone::Utc)
        );
        if unchanged {
            return Some(Self::Timed {
                start,
                end,
                occupied: schedule.occupied_dates(),
            });
        }
        let convert = |time| source.to_utc(time).map(|instant| zone.wall_time(instant));
        let start = convert(start)?;
        let end = convert(end)?;
        let last = if end.time() == NaiveTime::MIN && end.date() > start.date() {
            end.date().pred_opt()?
        } else {
            end.date()
        };
        let occupied = clipped_range(start.date().min(last), start.date().max(last))?;
        Some(Self::Timed {
            start,
            end,
            occupied,
        })
    }

    pub const fn start_date(&self) -> NaiveDate {
        match self {
            Self::AllDay(dates) => dates.start(),
            Self::Timed { start, .. } => start.date(),
        }
    }

    pub const fn occupied_dates(&self) -> DateRange {
        match self {
            Self::AllDay(dates)
            | Self::Timed {
                occupied: dates, ..
            } => *dates,
        }
    }

    pub const fn start_time(self) -> Option<NaiveTime> {
        match self {
            Self::AllDay(_) => None,
            Self::Timed { start, .. } => Some(start.time()),
        }
    }

    pub fn continues_after(self, now: NaiveDateTime) -> bool {
        match self {
            Self::AllDay(dates) => dates.end() >= now.date(),
            Self::Timed { end, .. } => end > now,
        }
    }
}

impl fmt::Display for DisplayedSchedule {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AllDay(dates) => dates.fmt(formatter),
            Self::Timed { start, end, .. } => {
                write!(formatter, "{}", start.format("%d.%m.%Y %H:%M"))?;
                if start != end {
                    let format = if start.date() == end.date() {
                        "%H:%M"
                    } else {
                        "%d.%m.%Y %H:%M"
                    };
                    write!(formatter, " — {}", end.format(format))?;
                }
                Ok(())
            }
        }
    }
}

impl EventSchedule {
    /// Даты для выделения в текущей зоне просмотра; полностью вышедшие за диапазон планера интервалы не проецируются.
    #[must_use]
    pub fn displayed_dates(self, zone: DisplayTimeZone) -> Option<DateRange> {
        DisplayedSchedule::project(self, zone).map(|schedule| schedule.occupied_dates())
    }
}

fn clipped_range(start: NaiveDate, end: NaiveDate) -> Option<DateRange> {
    DateRange::new(
        start.max(Year::clamped(i32::MIN).range().start()),
        end.min(Year::clamped(i32::MAX).range().end()),
    )
    .ok()
}

/// Разница двух зон может превысить сутки; запас не теряет экземпляры у границ года и выборки.
pub fn source_query(query: DateRange) -> DateRange {
    let padding = Duration::days(2);
    clipped_range(query.start() - padding, query.end() + padding)
        .expect("Expanded supported range is nonempty")
}
