//! Определения IANA-зон при экспорте охватывают поддерживаемый планером диапазон 1900–2100.

use std::{collections::BTreeMap, fmt, fmt::Write as _};

use chrono::{DateTime, Duration, NaiveDateTime, Offset as _, TimeZone as _, Utc};
use chrono_tz::{OffsetComponents as _, Tz};

fn offset(zone: Tz, time: DateTime<Utc>) -> i32 {
    zone.offset_from_utc_datetime(&time.naive_utc())
        .fix()
        .local_minus_utc()
}

struct UtcOffset(i32);

impl fmt::Display for UtcOffset {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let absolute = self.0.unsigned_abs();
        write!(
            formatter,
            "{}{:02}{:02}",
            if self.0 < 0 { '-' } else { '+' },
            absolute / 3600,
            absolute % 3600 / 60
        )?;
        if !absolute.is_multiple_of(60) {
            write!(formatter, "{:02}", absolute % 60)?;
        }
        Ok(())
    }
}

pub(super) fn append(output: &mut String, zone: Tz) {
    let start = Utc
        .with_ymd_and_hms(1900, 1, 1, 0, 0, 0)
        .single()
        .expect("Valid boundary");
    let end = Utc
        .with_ymd_and_hms(2101, 1, 1, 0, 0, 0)
        .single()
        .expect("Valid boundary");
    let initial = offset(zone, start);
    let mut transitions: BTreeMap<(i32, i32, bool), Vec<NaiveDateTime>> = BTreeMap::new();
    let daylight = |time: DateTime<Utc>| {
        zone.offset_from_utc_datetime(&time.naive_utc())
            .dst_offset()
            != Duration::zero()
    };
    transitions
        .entry((initial, initial, daylight(start)))
        .or_default()
        .push(start.with_timezone(&zone).naive_local());
    let mut day = start;
    let mut previous = initial;
    while day < end {
        let next = day + Duration::days(1);
        let current = offset(zone, next);
        if previous != current {
            let mut low = day.timestamp();
            let mut high = next.timestamp();
            while high - low > 1 {
                let middle = low + (high - low) / 2;
                let time = DateTime::from_timestamp(middle, 0).expect("Supported timestamp");
                if offset(zone, time) == previous {
                    low = middle;
                } else {
                    high = middle;
                }
            }
            let time = DateTime::from_timestamp(high, 0).expect("Supported timestamp");
            let local = time.naive_utc() + Duration::seconds(i64::from(previous));
            transitions
                .entry((previous, current, daylight(time)))
                .or_default()
                .push(local);
        }
        day = next;
        previous = current;
    }
    write!(output, "BEGIN:VTIMEZONE\r\nTZID:{}\r\n", zone.name()).expect("Writing to String");
    for ((from, to, daylight), dates) in transitions {
        let (first, remaining) = dates.split_first().expect("Nonempty transition group");
        let kind = if daylight { "DAYLIGHT" } else { "STANDARD" };
        write!(
            output,
            "BEGIN:{kind}\r\nDTSTART:{}\r\nTZOFFSETFROM:{}\r\nTZOFFSETTO:{}\r\n",
            first.format("%Y%m%dT%H%M%S"),
            UtcOffset(from),
            UtcOffset(to)
        )
        .expect("Writing to String");
        for date in remaining {
            write!(output, "RDATE:{}\r\n", date.format("%Y%m%dT%H%M%S"))
                .expect("Writing to String");
        }
        write!(output, "END:{kind}\r\n").expect("Writing to String");
    }
    output.push_str("END:VTIMEZONE\r\n");
}
