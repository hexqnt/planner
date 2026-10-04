use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use icalendar::parser::Property;

use crate::model::{DateRange, EventSchedule, EventTimeZone, EventTimes, OccurrenceStart};

use super::{Error, Result};

#[derive(Clone, Copy)]
pub(super) enum Start {
    Date(NaiveDate),
    Time(NaiveDateTime, EventTimeZone),
}

#[derive(Clone, Copy)]
enum ValueKind {
    Date,
    Time(EventTimeZone),
}

fn parameter<'a>(property: &'a Property<'_>, key: &str) -> Result<Option<&'a str>> {
    let mut values = property
        .params
        .iter()
        .filter(|parameter| parameter.key.as_str().eq_ignore_ascii_case(key));
    let value = values
        .next()
        .map(|parameter| {
            parameter
                .val
                .as_ref()
                .map(icalendar::parser::ParseString::as_str)
                .ok_or("Missing parameter value")
        })
        .transpose()?;
    if values.next().is_some() {
        return Err(Error(format!("Duplicate {key} parameter")));
    }
    Ok(value)
}

pub(super) fn parse(property: &Property<'_>) -> Result<Start> {
    ValueKind::parse(property)?.value(property.val.as_str())
}

impl ValueKind {
    fn parse(property: &Property<'_>) -> Result<Self> {
        let kind = parameter(property, "VALUE")?.unwrap_or("DATE-TIME");
        let timezone = parameter(property, "TZID")?;
        if kind == "DATE" {
            if timezone.is_some() {
                return Err("TZID is not allowed for DATE".into());
            }
            return Ok(Self::Date);
        }
        if kind != "DATE-TIME" {
            return Err("Unsupported date value type".into());
        }
        let zone = timezone.map_or(Ok(EventTimeZone::Floating), |name| {
            name.parse().map(EventTimeZone::Named).map_err(|_| {
                Error(format!(
                    "Unsupported TZID: {name}; expected an IANA timezone"
                ))
            })
        })?;
        Ok(Self::Time(zone))
    }

    fn value(self, value: &str) -> Result<Start> {
        let Self::Time(zone) = self else {
            return Ok(Start::Date(date(value)?));
        };
        let (value, zone) = if let Some(value) = value.strip_suffix('Z') {
            if zone != EventTimeZone::Floating {
                return Err("UTC date cannot have TZID".into());
            }
            (value, EventTimeZone::Utc)
        } else {
            (value, zone)
        };
        Ok(Start::Time(datetime(value)?, zone))
    }
}

pub(super) fn date(value: &str) -> Result<NaiveDate> {
    if value.len() != 8 || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("Invalid DATE; expected YYYYMMDD".into());
    }
    NaiveDate::parse_from_str(value, "%Y%m%d").map_err(|_| "Invalid DATE".into())
}

pub(super) fn datetime(value: &str) -> Result<NaiveDateTime> {
    if value.len() != 15
        || value.as_bytes().get(8) != Some(&b'T')
        || !value
            .bytes()
            .enumerate()
            .all(|(index, byte)| index == 8 || byte.is_ascii_digit())
    {
        return Err("Invalid DATE-TIME; expected YYYYMMDDTHHMMSS".into());
    }
    NaiveDateTime::parse_from_str(value, "%Y%m%dT%H%M%S").map_err(|_| "Invalid DATE-TIME".into())
}

pub(super) fn schedule(start: Start, end: Option<Start>) -> Result<EventSchedule> {
    match (start, end) {
        (Start::Date(start), end) => {
            let end = match end {
                None => start,
                Some(Start::Date(end)) if end > start => {
                    end.pred_opt().ok_or("DTEND is out of range")?
                }
                _ => return Err("All-day DTEND must be a later DATE".into()),
            };
            Ok(EventSchedule::all_day(DateRange::new(start, end)?))
        }
        (Start::Time(start, zone), end) => {
            let end = match end {
                None => start,
                Some(Start::Time(end, end_zone)) if zone == end_zone => end,
                Some(Start::Time(
                    end,
                    end_zone @ (EventTimeZone::Utc | EventTimeZone::Named(_)),
                )) if zone != EventTimeZone::Floating => {
                    let utc = end_zone
                        .to_utc(end)
                        .ok_or("Unable to resolve DTEND timezone")?;
                    let local = zone.wall_time(utc);
                    if zone.to_utc(local) != Some(utc) {
                        return Err("DTEND in the second occurrence of a repeated local hour is not supported".into());
                    }
                    local
                }
                _ => return Err("DTSTART and DTEND must use compatible timezones".into()),
            };
            Ok(EventSchedule::timed(
                DateRange::new(start.date(), end.date())?,
                EventTimes {
                    start: start.time(),
                    end: end.time(),
                },
            )?
            .with_timezone(zone)?)
        }
    }
}

pub(super) fn exclusions<'a>(
    property: &'a Property<'_>,
    start: Start,
) -> Result<impl Iterator<Item = Result<OccurrenceStart>> + 'a> {
    let kind = ValueKind::parse(property)?;
    Ok(property
        .val
        .as_str()
        .split(',')
        .map(move |value| exclusion(kind.value(value)?, start)))
}

fn exclusion(value: Start, start: Start) -> Result<OccurrenceStart> {
    match (value, start) {
        (Start::Date(date), Start::Date(_)) => Ok(OccurrenceStart::Date(date)),
        (Start::Time(time, EventTimeZone::Floating), Start::Time(_, EventTimeZone::Floating)) => {
            Ok(OccurrenceStart::Local(time))
        }
        (
            Start::Time(time, zone @ (EventTimeZone::Utc | EventTimeZone::Named(_))),
            Start::Time(_, EventTimeZone::Utc | EventTimeZone::Named(_)),
        ) => Ok(OccurrenceStart::Instant(
            zone.to_utc(time)
                .ok_or("Unable to resolve EXDATE timezone")?,
        )),
        _ => Err("EXDATE must match DTSTART value type and timezone".into()),
    }
}

pub(super) fn exclusion_property(start: OccurrenceStart) -> icalendar::Property {
    match start {
        OccurrenceStart::Date(date) => {
            icalendar::Property::new("EXDATE", date.format("%Y%m%d").to_string())
                .add_parameter("VALUE", "DATE")
                .done()
        }
        OccurrenceStart::Local(time) => {
            icalendar::Property::new("EXDATE", time.format("%Y%m%dT%H%M%S").to_string())
        }
        OccurrenceStart::Instant(time) => {
            icalendar::Property::new("EXDATE", time.format("%Y%m%dT%H%M%SZ").to_string())
        }
    }
}

pub(super) fn property(
    name: &str,
    time: NaiveDateTime,
    schedule: EventSchedule,
) -> icalendar::Property {
    let format = if schedule.times().is_none() {
        "%Y%m%d"
    } else if schedule.timezone() == EventTimeZone::Utc {
        "%Y%m%dT%H%M%SZ"
    } else {
        "%Y%m%dT%H%M%S"
    };
    let mut property = icalendar::Property::new(name, time.format(format).to_string());
    if schedule.times().is_none() {
        property.add_parameter("VALUE", "DATE");
    }
    if let EventTimeZone::Named(zone) = schedule.timezone() {
        property.add_parameter("TZID", zone.name());
    }
    property
}

pub(super) fn properties(
    schedule: EventSchedule,
) -> Result<impl Iterator<Item = icalendar::Property>> {
    let dates = schedule.dates();
    let (start, end) = if let Some(times) = schedule.times() {
        (
            dates.start().and_time(times.start),
            dates.end().and_time(times.end),
        )
    } else {
        (
            dates.start().and_time(NaiveTime::MIN),
            dates
                .end()
                .succ_opt()
                .ok_or("DTEND is out of range")?
                .and_time(NaiveTime::MIN),
        )
    };
    // Равный DTSTART конец запрещён RFC 5545; отсутствие DTEND означает нулевую длительность.
    let end = (start != end).then(|| property("DTEND", end, schedule));
    Ok([Some(property("DTSTART", start, schedule)), end]
        .into_iter()
        .flatten())
}
