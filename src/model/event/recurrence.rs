use std::num::{NonZeroU16, NonZeroU32};

use chrono::{DateTime, Datelike as _, Days, NaiveDate, NaiveDateTime, NaiveTime, Utc, Weekday};
use serde::{Deserialize, Serialize};

use crate::model::{DateRange, InputError, Year};

use super::{EventSchedule, EventTimeZone};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Frequency {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

/// Непустой набор дней недели; младший бит соответствует понедельнику.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub struct Weekdays(u8);

impl TryFrom<u8> for Weekdays {
    type Error = InputError;

    fn try_from(bits: u8) -> Result<Self, Self::Error> {
        if bits == 0 || bits & !0x7f != 0 {
            return Err(InputError::RecurrencePattern);
        }
        Ok(Self(bits))
    }
}

impl From<Weekdays> for u8 {
    fn from(days: Weekdays) -> Self {
        days.0
    }
}

impl Weekdays {
    pub const fn contains(self, day: Weekday) -> bool {
        self.0 & (1 << day.num_days_from_monday()) != 0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "RecurrenceInput", into = "RecurrenceInput")]
pub struct Recurrence {
    frequency: Frequency,
    interval: NonZeroU16,
    end: RecurrenceEnd,
    weekdays: Option<Weekdays>,
    week_start: Weekday,
}

/// Одновременно задать количество и границу окончания невозможно.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RecurrenceEnd {
    Never,
    Count(NonZeroU32),
    Until { date: NaiveDate, cutoff: Cutoff },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cutoff {
    Date,
    Local(NaiveTime),
    Instant {
        local: NaiveTime,
        utc: DateTime<Utc>,
    },
}

impl Recurrence {
    /// # Errors
    /// Возвращает ошибку, если конечная дата выходит за поддерживаемые годы.
    pub fn new(
        frequency: Frequency,
        interval: NonZeroU16,
        until: Option<NaiveDate>,
    ) -> Result<Self, InputError> {
        if let Some(date) = until {
            Year::try_from(date.year())?;
        }
        Ok(Self {
            frequency,
            interval,
            end: until.map_or(RecurrenceEnd::Never, |date| RecurrenceEnd::Until {
                date,
                cutoff: Cutoff::Date,
            }),
            weekdays: None,
            week_start: Weekday::Mon,
        })
    }

    #[must_use]
    pub const fn frequency(self) -> Frequency {
        self.frequency
    }

    #[must_use]
    pub const fn interval(self) -> NonZeroU16 {
        self.interval
    }

    #[must_use]
    pub const fn until(self) -> Option<NaiveDate> {
        match self.end {
            RecurrenceEnd::Until { date, .. } => Some(date),
            RecurrenceEnd::Never | RecurrenceEnd::Count(_) => None,
        }
    }

    /// # Errors
    /// Возвращает ошибку для несовместимых ограничений, частоты и дней недели.
    pub fn with_pattern(
        mut self,
        count: Option<NonZeroU32>,
        weekdays: Option<Weekdays>,
        week_start: Weekday,
        until_time: Option<NaiveTime>,
    ) -> Result<Self, InputError> {
        if weekdays.is_some() && self.frequency != Frequency::Weekly {
            return Err(InputError::RecurrencePattern);
        }
        self.end = match (self.until(), count, until_time) {
            (None, None, None) => RecurrenceEnd::Never,
            (None, Some(count), None) => RecurrenceEnd::Count(count),
            (Some(date), None, time) => RecurrenceEnd::Until {
                date,
                cutoff: time.map_or(Cutoff::Date, Cutoff::Local),
            },
            _ => return Err(InputError::RecurrencePattern),
        };
        self.weekdays = weekdays;
        self.week_start = week_start;
        Ok(self)
    }

    #[must_use]
    pub const fn count(self) -> Option<NonZeroU32> {
        match self.end {
            RecurrenceEnd::Count(count) => Some(count),
            RecurrenceEnd::Never | RecurrenceEnd::Until { .. } => None,
        }
    }

    #[must_use]
    pub const fn weekdays(self) -> Option<Weekdays> {
        self.weekdays
    }

    #[must_use]
    pub const fn week_start(self) -> Weekday {
        self.week_start
    }

    #[must_use]
    pub const fn until_time(self) -> Option<NaiveTime> {
        match self.end {
            RecurrenceEnd::Until {
                cutoff: Cutoff::Local(time) | Cutoff::Instant { local: time, .. },
                ..
            } => Some(time),
            _ => None,
        }
    }

    /// Абсолютная граница сохраняет точность UNTIL в неоднозначный час перевода часов.
    /// # Errors
    /// Возвращает ошибку, если UTC-граница не соответствует локальному окончанию повторов.
    pub const fn with_until_instant(
        mut self,
        instant: Option<DateTime<Utc>>,
    ) -> Result<Self, InputError> {
        match (self.end, instant) {
            (
                RecurrenceEnd::Until {
                    date,
                    cutoff: Cutoff::Local(local) | Cutoff::Instant { local, .. },
                },
                Some(utc),
            ) => {
                self.end = RecurrenceEnd::Until {
                    date,
                    cutoff: Cutoff::Instant { local, utc },
                };
            }
            (
                RecurrenceEnd::Until {
                    date,
                    cutoff: Cutoff::Instant { local, .. },
                },
                None,
            ) => {
                self.end = RecurrenceEnd::Until {
                    date,
                    cutoff: Cutoff::Local(local),
                };
            }
            (_, Some(_)) => return Err(InputError::RecurrencePattern),
            (_, None) => {}
        }
        Ok(self)
    }

    #[must_use]
    pub const fn until_instant(self) -> Option<DateTime<Utc>> {
        match self.end {
            RecurrenceEnd::Until {
                cutoff: Cutoff::Instant { utc, .. },
                ..
            } => Some(utc),
            _ => None,
        }
    }

    #[must_use]
    pub fn allows_start(self, time: NaiveDateTime, zone: EventTimeZone) -> bool {
        match self.end {
            RecurrenceEnd::Never | RecurrenceEnd::Count(_) => true,
            RecurrenceEnd::Until {
                cutoff: Cutoff::Instant { utc, .. },
                ..
            } => zone.to_utc(time).is_some_and(|time| time <= utc),
            RecurrenceEnd::Until {
                date,
                cutoff: Cutoff::Date,
            } => time.date() <= date,
            RecurrenceEnd::Until {
                date,
                cutoff: Cutoff::Local(until),
            } => time <= date.and_time(until),
        }
    }

    pub(super) fn validate_for(self, schedule: EventSchedule) -> Result<(), InputError> {
        let start = schedule.start();
        if self
            .weekdays
            .is_some_and(|days| !days.contains(start.weekday()))
            || (schedule.times().is_none() && self.until_time().is_some())
        {
            return Err(InputError::RecurrencePattern);
        }
        if let Some(instant) = self.until_instant() {
            let zone = schedule.timezone();
            let local = zone.wall_time(instant);
            if zone == EventTimeZone::Floating
                || Some(local.date()) != self.until()
                || Some(local.time()) != self.until_time()
            {
                return Err(InputError::RecurrencePattern);
            }
        }
        if !self.allows_start(start, schedule.timezone()) {
            return Err(InputError::RecurrenceUntil);
        }
        Ok(())
    }

    fn distance(self, start: NaiveDate, end: NaiveDate) -> u32 {
        let distance = match self.frequency {
            Frequency::Daily => end.signed_duration_since(start).num_days(),
            Frequency::Weekly => end.signed_duration_since(start).num_days() / 7,
            Frequency::Monthly => {
                i64::from(end.year() - start.year()) * 12 + i64::from(end.month())
                    - i64::from(start.month())
            }
            Frequency::Yearly => i64::from(end.year() - start.year()),
        };
        u32::try_from(distance.max(0)).expect("Supported date range fits u32")
            / u32::from(self.interval.get())
    }

    fn date_at(self, start: NaiveDate, step: u32) -> Option<NaiveDate> {
        let offset = step.checked_mul(u32::from(self.interval.get()))?;
        match self.frequency {
            Frequency::Daily => start.checked_add_days(Days::new(u64::from(offset))),
            Frequency::Weekly => start.checked_add_days(Days::new(u64::from(offset) * 7)),
            Frequency::Monthly => {
                let month = start.month0().checked_add(offset)?;
                let year = start.year().checked_add(i32::try_from(month / 12).ok()?)?;
                NaiveDate::from_ymd_opt(year, month % 12 + 1, start.day())
            }
            Frequency::Yearly => NaiveDate::from_ymd_opt(
                start.year().checked_add(i32::try_from(offset).ok()?)?,
                start.month(),
                start.day(),
            ),
        }
    }
}

impl TryFrom<RecurrenceInput> for Recurrence {
    type Error = InputError;
    fn try_from(value: RecurrenceInput) -> Result<Self, Self::Error> {
        Self::new(value.frequency, value.interval, value.until)?
            .with_pattern(
                value.count,
                value.weekdays,
                value.week_start,
                value.until_time,
            )?
            .with_until_instant(value.until_instant)
    }
}

#[derive(Serialize, Deserialize)]
struct RecurrenceInput {
    #[serde(default)]
    until_instant: Option<DateTime<Utc>>,
    #[serde(default)]
    count: Option<NonZeroU32>,
    #[serde(default)]
    weekdays: Option<Weekdays>,
    #[serde(default = "monday")]
    week_start: Weekday,
    #[serde(default)]
    until_time: Option<NaiveTime>,
    frequency: Frequency,
    interval: NonZeroU16,
    until: Option<NaiveDate>,
}

const fn monday() -> Weekday {
    Weekday::Mon
}

impl From<Recurrence> for RecurrenceInput {
    fn from(value: Recurrence) -> Self {
        Self {
            until_instant: value.until_instant(),
            count: value.count(),
            weekdays: value.weekdays,
            week_start: value.week_start,
            until_time: value.until_time(),
            frequency: value.frequency,
            interval: value.interval,
            until: value.until(),
        }
    }
}

impl EventSchedule {
    /// Возвращает пересекающие запрос экземпляры без правила повтора; перебор начинается около запроса, несуществующие даты пропускаются.
    /// # Panics
    /// Паника означает нарушение внутреннего инварианта поддерживаемого диапазона дат.
    pub fn occurrences(self, query: DateRange) -> impl Iterator<Item = Self> {
        let anchor = self.dates().start();
        let duration = self.dates().end().signed_duration_since(anchor);
        let earliest = query
            .start()
            .checked_sub_signed(duration)
            .unwrap_or(anchor)
            .max(anchor);
        let recurrence = self.recurrence();
        let query_end = query.end();
        let limit = recurrence
            .and_then(Recurrence::until)
            .map_or(query_end, |until| until.min(query_end));
        let by_weekday = recurrence.is_some_and(|rule| rule.weekdays.is_some());
        let distance = |date: NaiveDate| {
            if by_weekday {
                u32::try_from(date.signed_duration_since(anchor).num_days().max(0))
                    .expect("Supported date range fits u32")
            } else {
                recurrence.map_or(0, |rule| rule.distance(anchor, date))
            }
        };
        let first = if recurrence.is_some_and(|rule| rule.count().is_some()) {
            0
        } else {
            distance(earliest)
        };
        let last = distance(limit);
        let count = recurrence
            .and_then(Recurrence::count)
            .map_or(usize::MAX, |count| {
                usize::try_from(count.get()).expect("u32 count fits usize")
            });
        let start_time = self.start().time();
        let anchor_weekday = recurrence.map_or(0, |rule| {
            (anchor.weekday().num_days_from_monday() + 7 - rule.week_start.num_days_from_monday())
                % 7
        });
        (first..=last)
            .filter_map(move |step| {
                let start = if by_weekday {
                    let rule = recurrence?;
                    let date = anchor.checked_add_days(Days::new(u64::from(step)))?;
                    let week = (step + anchor_weekday) / 7;
                    if !week.is_multiple_of(u32::from(rule.interval.get()))
                        || !rule.weekdays?.contains(date.weekday())
                    {
                        return None;
                    }
                    date
                } else {
                    recurrence.map_or(Some(anchor), |rule| rule.date_at(anchor, step))?
                };
                if start > limit
                    || recurrence.is_some_and(|rule| {
                        !rule.allows_start(start.and_time(start_time), self.timezone())
                    })
                {
                    return None;
                }
                // Несуществующие часы повторений не входят в COUNT; исходный DTSTART сохраняется.
                if start != anchor && !self.timezone().exists(start.and_time(start_time)) {
                    return None;
                }
                let end = start.checked_add_signed(duration)?;
                let dates = DateRange::new(start, end).ok()?;
                let schedule = match self.times() {
                    Some(times) => Self::timed(dates, times).ok()?,
                    None => Self::all_day(dates),
                };
                schedule.with_timezone(self.timezone()).ok()
            })
            .take(count)
            .filter(move |schedule| schedule.occupied_dates().overlaps(query))
    }
}

#[cfg(test)]
mod tests;
