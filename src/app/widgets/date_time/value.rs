use chrono::{Days, Months, NaiveDate, NaiveTime, TimeDelta, Timelike as _};

use crate::text::Language;

pub(in crate::app) fn parse_date(text: &str) -> Result<NaiveDate, chrono::ParseError> {
    let text = text.trim();
    NaiveDate::parse_from_str(
        text,
        if text.contains('.') {
            "%d.%m.%Y"
        } else {
            "%Y-%m-%d"
        },
    )
}

pub(in crate::app) fn parse_time(text: &str) -> Result<NaiveTime, chrono::ParseError> {
    let text = text.trim();
    if matches!(text.len(), 3 | 4) && text.bytes().all(|byte| byte.is_ascii_digit()) {
        let (hours, minutes) = text.split_at(text.len() - 2);
        if let (Ok(hours), Ok(minutes)) = (hours.parse(), minutes.parse())
            && let Some(time) = NaiveTime::from_hms_opt(hours, minutes, 0)
        {
            return Ok(time);
        }
    }
    NaiveTime::parse_from_str(
        text,
        if text.bytes().filter(|&byte| byte == b':').count() == 1 {
            "%H:%M"
        } else {
            "%H:%M:%S%.f"
        },
    )
}

pub(in crate::app) fn format_time(time: NaiveTime) -> String {
    time.format(if time.second() == 0 && time.nanosecond() == 0 {
        "%H:%M"
    } else {
        "%H:%M:%S%.f"
    })
    .to_string()
}

#[derive(Clone, Copy)]
pub(super) enum Value {
    Date(NaiveDate),
    Time(NaiveTime),
}

impl Value {
    pub(super) fn format(self, language: Language) -> String {
        match self {
            Self::Date(date) => date
                .format(language.text("%d.%m.%Y", "%Y-%m-%d"))
                .to_string(),
            Self::Time(time) => format_time(time),
        }
    }

    pub(super) fn step(self, text: &str, cursor: usize, forward: bool) -> Option<Self> {
        let segment = if matches!(self, Self::Time(_)) && !text.contains(':') {
            usize::from(cursor > text.len().saturating_sub(2))
        } else {
            text.chars()
                .take(cursor)
                .filter(|&c| c == '.' || c == '-' || c == ':')
                .count()
        };
        match self {
            Self::Date(date) => {
                let segment = if text.contains('-') {
                    2_usize.saturating_sub(segment)
                } else {
                    segment
                };
                let date = match (segment, forward) {
                    (0, true) => date.checked_add_days(Days::new(1)),
                    (0, false) => date.checked_sub_days(Days::new(1)),
                    (1, true) => date.checked_add_months(Months::new(1)),
                    (1, false) => date.checked_sub_months(Months::new(1)),
                    (_, true) => date.checked_add_months(Months::new(12)),
                    (_, false) => date.checked_sub_months(Months::new(12)),
                }?;
                Some(Self::Date(date))
            }
            Self::Time(time) => {
                let seconds = match segment {
                    0 => 3600,
                    1 => 60,
                    _ => 1,
                };
                Some(Self::Time(
                    time.overflowing_add_signed(TimeDelta::seconds(if forward {
                        seconds
                    } else {
                        -seconds
                    }))
                    .0,
                ))
            }
        }
    }
}
