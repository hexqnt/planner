use chrono::{Datelike as _, NaiveDate, Weekday};
use holidays_ru::{DayFlags, Federal, flags, flags_with_region, regions};
use serde::{Deserialize, Serialize};

use crate::{model::Year, text::Language};

#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Region {
    Naive,
    #[default]
    Federal,
    Tatarstan,
    Bashkortostan,
    Crimea,
    Adygea,
    Dagestan,
    Chuvashia,
    Buryatia,
}

impl Region {
    pub const ALL: [Self; 9] = [
        Self::Naive,
        Self::Federal,
        Self::Tatarstan,
        Self::Bashkortostan,
        Self::Crimea,
        Self::Adygea,
        Self::Dagestan,
        Self::Chuvashia,
        Self::Buryatia,
    ];

    pub const fn name(self, language: Language) -> &'static str {
        let (ru, en) = match self {
            Self::Naive => ("Наивный (Сб/Вс)", "Naive (Sat/Sun)"),
            Self::Federal => ("Федеральный", "Federal"),
            Self::Tatarstan => ("Татарстан", "Tatarstan"),
            Self::Bashkortostan => ("Башкортостан", "Bashkortostan"),
            Self::Crimea => ("Крым", "Crimea"),
            Self::Adygea => ("Адыгея", "Adygea"),
            Self::Dagestan => ("Дагестан", "Dagestan"),
            Self::Chuvashia => ("Чувашия", "Chuvashia"),
            Self::Buryatia => ("Бурятия", "Buryatia"),
        };
        language.text(ru, en)
    }

    pub(crate) fn day(self, date: NaiveDate) -> Day {
        let resolved = match self {
            Self::Naive => {
                let flags = match date.weekday() {
                    Weekday::Sat | Weekday::Sun => DayFlags::WEEKEND.with(DayFlags::DAY_OFF),
                    _ => DayFlags::WORKING_DAY,
                };
                return Day {
                    date,
                    flags,
                    predicted: false,
                };
            }
            Self::Federal => flags::<Federal, _>(date),
            Self::Tatarstan => flags_with_region::<regions::Tatarstan, _>(date),
            Self::Bashkortostan => flags_with_region::<regions::Bashkortostan, _>(date),
            Self::Crimea => flags_with_region::<regions::Crimea, _>(date),
            Self::Adygea => flags_with_region::<regions::Adygea, _>(date),
            Self::Dagestan => flags_with_region::<regions::Dagestan, _>(date),
            Self::Chuvashia => flags_with_region::<regions::Chuvashia, _>(date),
            Self::Buryatia => flags_with_region::<regions::Buryatia, _>(date),
        }
        .expect("Date is within the supported calendar range");
        Day {
            date,
            flags: resolved.value(),
            predicted: resolved.is_predict(),
        }
    }
}

#[derive(Clone, Copy)]
pub struct Day {
    pub date: NaiveDate,
    pub flags: DayFlags,
    pub predicted: bool,
}

pub struct Month {
    pub offset: u32,
    pub days: Vec<Day>,
}

/// Системный календарь вычисляется только при смене года или региона.
pub struct Calendar {
    pub year: Year,
    pub region: Region,
    pub months: [Month; 12],
}

impl Calendar {
    pub fn new(year: Year, region: Region) -> Self {
        Self {
            year,
            region,
            months: std::array::from_fn(|index| {
                let month = u32::try_from(index + 1).expect("Month fits u32");
                let first = NaiveDate::from_ymd_opt(year.get(), month, 1).expect("Valid month");
                let days = first
                    .iter_days()
                    .take_while(|date| date.month() == month)
                    .map(|date| region.day(date))
                    .collect();
                Month {
                    offset: first.weekday().num_days_from_monday(),
                    days,
                }
            }),
        }
    }

    pub fn refresh(&mut self, year: Year, region: Region) {
        if self.year != year || self.region != region {
            *self = Self::new(year, region);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leap_year_and_monday_first() {
        let calendar = Calendar::new(Year::try_from(2024).unwrap(), Region::Federal);
        assert_eq!(calendar.months[1].days.len(), 29);
        assert_eq!(calendar.months[0].offset, 0);
        assert_eq!(
            calendar.months.iter().map(|m| m.days.len()).sum::<usize>(),
            366
        );
    }

    #[test]
    fn regional_overlay_and_transferred_workdays() {
        let year = Year::try_from(2026).unwrap();
        let federal = Calendar::new(year, Region::Federal);
        let tatarstan = Calendar::new(year, Region::Tatarstan);
        assert!(federal.months[10].days[5].flags.is_working_day());
        assert!(tatarstan.months[10].days[5].flags.is_day_off());
        let date = NaiveDate::from_ymd_opt(2024, 4, 27).unwrap();
        assert!(Region::Federal.day(date).flags.is_working_day());
    }

    #[test]
    fn naive_calendar_ignores_holidays_transfers_and_predictions() {
        for year in [1900, 2024, 2026, 2100] {
            let calendar = Calendar::new(Year::try_from(year).unwrap(), Region::Naive);
            for day in calendar.months.iter().flat_map(|month| &month.days) {
                let weekend = matches!(day.date.weekday(), Weekday::Sat | Weekday::Sun);
                assert_eq!(day.flags.is_day_off(), weekend);
                assert_eq!(day.flags.is_weekend(), weekend);
                assert_eq!(day.flags.is_working_day(), !weekend);
                assert!(!day.flags.is_holiday());
                assert!(!day.flags.is_transferred());
                assert!(!day.flags.is_short_day());
                assert!(!day.predicted);
            }
        }
        let mut calendar = Calendar::new(Year::try_from(2024).unwrap(), Region::Federal);
        assert!(calendar.months[0].days[0].flags.is_day_off());
        assert!(calendar.months[3].days[26].flags.is_working_day());
        calendar.refresh(calendar.year, Region::Naive);
        assert!(calendar.months[0].days[0].flags.is_working_day());
        assert!(calendar.months[3].days[26].flags.is_day_off());
    }

    #[test]
    fn all_supported_regions_and_year_boundaries() {
        for year in [1900, 2026, 2100] {
            for region in Region::ALL {
                let calendar = Calendar::new(Year::try_from(year).unwrap(), region);
                assert_eq!(calendar.months.len(), 12);
                assert_eq!(calendar.months[11].days.last().unwrap().date.day(), 31);
            }
        }
    }
}
