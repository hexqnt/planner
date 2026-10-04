use std::{
    fmt::Write as _,
    num::{NonZeroU16, NonZeroU32},
};

use chrono::{NaiveTime, Weekday};

use crate::model::{EventSchedule, EventTimeZone, Frequency, Recurrence, Weekdays};

use super::{Error, Result, dates};

#[derive(Clone, Copy)]
#[repr(u8)]
enum RulePart {
    Frequency = 1,
    Interval = 2,
    Count = 4,
    Until = 8,
    Weekdays = 16,
    WeekStart = 32,
}

impl std::str::FromStr for RulePart {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "FREQ" => Ok(Self::Frequency),
            "INTERVAL" => Ok(Self::Interval),
            "COUNT" => Ok(Self::Count),
            "UNTIL" => Ok(Self::Until),
            "BYDAY" => Ok(Self::Weekdays),
            "WKST" => Ok(Self::WeekStart),
            _ => Err(Error(format!("Unsupported RRULE part: {value}"))),
        }
    }
}

const WEEKDAYS: [(Weekday, &str); 7] = [
    (Weekday::Mon, "MO"),
    (Weekday::Tue, "TU"),
    (Weekday::Wed, "WE"),
    (Weekday::Thu, "TH"),
    (Weekday::Fri, "FR"),
    (Weekday::Sat, "SA"),
    (Weekday::Sun, "SU"),
];

fn weekday(value: &str) -> Result<Weekday> {
    WEEKDAYS
        .iter()
        .find_map(|&(day, name)| (value == name).then_some(day))
        .ok_or_else(|| "Unsupported BYDAY or WKST; expected MO..SU without ordinals".into())
}

pub(super) fn parse(value: &str, schedule: EventSchedule) -> Result<Recurrence> {
    let mut frequency = None;
    let mut interval = NonZeroU16::MIN;
    let mut until = None;
    let mut until_time = None;
    let mut until_instant = None;
    let mut count = None;
    let mut weekdays = None;
    let mut week_start = Weekday::Mon;
    let mut seen = 0_u8;
    for part in value.split(';') {
        let (key, value) = part.split_once('=').ok_or("Invalid RRULE")?;
        let part: RulePart = key.parse()?;
        let bit = part as u8;
        if seen & bit != 0 {
            return Err(Error(format!("Duplicate RRULE part: {key}")));
        }
        seen |= bit;
        match part {
            RulePart::Frequency => {
                frequency = Some(match value {
                    "DAILY" => Frequency::Daily,
                    "WEEKLY" => Frequency::Weekly,
                    "MONTHLY" => Frequency::Monthly,
                    "YEARLY" => Frequency::Yearly,
                    _ => return Err(Error(format!("Unsupported FREQ: {value}"))),
                });
            }
            RulePart::Interval => {
                interval = value
                    .parse::<NonZeroU16>()
                    .map_err(|_| "Invalid INTERVAL")?;
            }
            RulePart::Count => {
                count = Some(value.parse::<NonZeroU32>().map_err(|_| "Invalid COUNT")?);
            }
            RulePart::Until => {
                if schedule.times().is_none() {
                    until = Some(dates::date(value)?);
                } else {
                    let zone = schedule.timezone();
                    let time = match (value.strip_suffix('Z'), zone) {
                        (Some(value), EventTimeZone::Utc | EventTimeZone::Named(_)) => {
                            let utc = dates::datetime(value)?.and_utc();
                            until_instant = Some(utc);
                            zone.wall_time(utc)
                        },
                        (None, EventTimeZone::Floating) => dates::datetime(value)?,
                        _ => return Err("UNTIL must match DTSTART: UTC for zoned events, local for floating events, DATE for all-day events".into()),
                    };
                    until = Some(time.date());
                    until_time = Some(time.time());
                }
            }
            RulePart::Weekdays => {
                let bits = value.split(',').try_fold(0, |bits, value| -> Result<u8> {
                    Ok(bits | (1 << weekday(value)?.num_days_from_monday()))
                })?;
                weekdays = Some(Weekdays::try_from(bits)?);
            }
            RulePart::WeekStart => week_start = weekday(value)?,
        }
    }
    Ok(
        Recurrence::new(frequency.ok_or("Missing FREQ")?, interval, until)?
            .with_pattern(count, weekdays, week_start, until_time)?
            .with_until_instant(until_instant)?,
    )
}

pub(super) fn format(rule: Recurrence, schedule: EventSchedule) -> Result<String> {
    let mut output = format!(
        "FREQ={};INTERVAL={}",
        match rule.frequency() {
            Frequency::Daily => "DAILY",
            Frequency::Weekly => "WEEKLY",
            Frequency::Monthly => "MONTHLY",
            Frequency::Yearly => "YEARLY",
        },
        rule.interval()
    );
    if let Some(count) = rule.count() {
        write!(output, ";COUNT={count}").expect("Writing to String");
    }
    if let Some(until) = rule.until() {
        let (time, format) = if let Some(instant) = rule.until_instant() {
            (instant.naive_utc(), "%Y%m%dT%H%M%SZ")
        } else if schedule.times().is_none() {
            (until.and_time(NaiveTime::MIN), "%Y%m%d")
        } else {
            let time = until.and_time(
                rule.until_time()
                    .unwrap_or(NaiveTime::from_hms_opt(23, 59, 59).expect("Valid time")),
            );
            match schedule.timezone() {
                EventTimeZone::Floating => (time, "%Y%m%dT%H%M%S"),
                zone => (
                    zone.to_utc(time)
                        .ok_or("Unable to resolve UNTIL timezone")?
                        .naive_utc(),
                    "%Y%m%dT%H%M%SZ",
                ),
            }
        };
        write!(output, ";UNTIL={}", time.format(format)).expect("Writing to String");
    }
    if let Some(days) = rule.weekdays() {
        output.push_str(";BYDAY=");
        let mut separator = "";
        for (day, name) in WEEKDAYS {
            if days.contains(day) {
                output.push_str(separator);
                output.push_str(name);
                separator = ",";
            }
        }
    }
    if rule.week_start() != Weekday::Mon {
        let name = WEEKDAYS
            .iter()
            .find(|&&(day, _)| day == rule.week_start())
            .expect("All weekdays")
            .1;
        write!(output, ";WKST={name}").expect("Writing to String");
    }
    Ok(output)
}
