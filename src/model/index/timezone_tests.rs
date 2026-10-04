use super::*;
use crate::model::EventTimeZone;

fn document(properties: &str, year: i32, zone: DisplayTimeZone) -> Document {
    let mut document = Document {
        year: Year::try_from(year).unwrap(),
        display_timezone: zone,
        ..Document::default()
    };
    let file = format!(
        "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:zone-test\r\n{properties}END:VEVENT\r\nEND:VCALENDAR\r\n"
    );
    document.merge_events(crate::ical::import(&file, CategoryId(1)).unwrap());
    document
}

fn labels(index: &mut EventIndex, document: &Document) -> Vec<String> {
    index
        .list(document, document.year.range())
        .map(|row| row.occurrence.date_label().to_owned())
        .collect()
}

#[test]
fn zone_changes_reproject_days_labels_and_selection_without_changing_events() {
    let mut document = document(
        "DTSTART:20261003T233000Z\r\nDTEND:20261004T003000Z\r\n",
        2026,
        DisplayTimeZone::Utc,
    );
    let original = serde_json::to_value(&document.events).unwrap();
    let mut index = EventIndex::default();
    assert_eq!(
        labels(&mut index, &document),
        ["03.10.2026 23:30 — 04.10.2026 00:30"]
    );
    assert_eq!(index.months[9].days[2].colors().len(), 1);
    document.display_timezone = DisplayTimeZone::Named(chrono_tz::Europe::Moscow);
    assert_eq!(labels(&mut index, &document), ["04.10.2026 02:30 — 03:30"]);
    assert_eq!(index.months[9].days[2].colors(), [] as [[u8; 3]; 0]);
    assert_eq!(index.months[9].days[3].colors().len(), 1);
    let day = "2026-10-04".parse().unwrap();
    assert_eq!(
        document.events[0]
            .schedule
            .displayed_dates(document.display_timezone),
        Some(DateRange::between(day, day))
    );
    let row = &index.months[9].events[0];
    assert_eq!(row.date_label(&index), "04.10.2026 02:30 — 03:30");
    assert!(std::ptr::eq(
        row.source_event(&document),
        &raw const document.events[0]
    ));
    assert_eq!(serde_json::to_value(&document.events).unwrap(), original);
}

#[test]
fn projection_includes_neighboring_years_and_offsets_more_than_a_day_apart() {
    for (properties, zone, year, label) in [
        (
            "DTSTART:20261231T233000Z\r\n",
            chrono_tz::Europe::Moscow,
            2027,
            "01.01.2027 02:30",
        ),
        (
            "DTSTART:20270101T003000Z\r\n",
            chrono_tz::America::New_York,
            2026,
            "31.12.2026 19:30",
        ),
        (
            "DTSTART;TZID=Pacific/Pago_Pago:20261230T233000\r\n",
            chrono_tz::Pacific::Kiritimati,
            2027,
            "01.01.2027 00:30",
        ),
    ] {
        let document = document(properties, year, DisplayTimeZone::Named(zone));
        assert_eq!(labels(&mut EventIndex::default(), &document), [label]);
    }
}

#[test]
fn all_day_and_floating_events_keep_their_dates_and_wall_clocks() {
    for (properties, label) in [
        (
            "DTSTART;VALUE=DATE:20261003\r\nDTEND;VALUE=DATE:20261005\r\n",
            "03.10.2026 — 04.10.2026",
        ),
        (
            "DTSTART:20261003T233000\r\nDTEND:20261004T003000\r\n",
            "03.10.2026 23:30 — 04.10.2026 00:30",
        ),
    ] {
        let mut document = document(properties, 2026, DisplayTimeZone::Utc);
        let mut index = EventIndex::default();
        for zone in [
            DisplayTimeZone::Utc,
            DisplayTimeZone::Named(chrono_tz::Pacific::Kiritimati),
            DisplayTimeZone::Named(chrono_tz::America::New_York),
        ] {
            document.display_timezone = zone;
            assert_eq!(labels(&mut index, &document), [label]);
        }
    }
}

#[test]
fn recurrence_and_exclusions_are_resolved_in_the_source_zone_before_projection() {
    let document = document(
        "DTSTART;TZID=America/New_York:20260307T090000\r\nDTEND;TZID=America/New_York:20260307T100000\r\nRRULE:FREQ=DAILY;COUNT=3\r\nEXDATE;TZID=America/New_York:20260308T090000\r\n",
        2026,
        DisplayTimeZone::Utc,
    );
    assert_eq!(
        labels(&mut EventIndex::default(), &document),
        ["07.03.2026 14:00 — 15:00", "09.03.2026 13:00 — 14:00"]
    );
    assert_eq!(
        document.events[0].schedule.timezone(),
        EventTimeZone::Named(chrono_tz::America::New_York)
    );
    let exported = crate::ical::export(&document).unwrap();
    let imported = crate::ical::import(&exported, CategoryId(1)).unwrap();
    assert_eq!(imported[0].schedule, document.events[0].schedule);
    assert_eq!(
        imported[0].identity.exclusions,
        document.events[0].identity.exclusions
    );
}

#[test]
fn projected_midnight_excludes_the_last_day_and_backward_clocks_remain_visible() {
    let midnight = document(
        "DTSTART:20261003T203000Z\r\nDTEND:20261003T210000Z\r\n",
        2026,
        DisplayTimeZone::Named(chrono_tz::Europe::Moscow),
    );
    let mut index = EventIndex::default();
    index.refresh(&midnight);
    assert_eq!(index.months[9].days[2].colors().len(), 1);
    assert_eq!(index.months[9].days[3].colors(), [] as [[u8; 3]; 0]);
    let backward = document(
        "DTSTART:20261025T004500Z\r\nDTEND:20261025T011500Z\r\n",
        2026,
        DisplayTimeZone::Named(chrono_tz::Europe::Berlin),
    );
    assert_eq!(
        labels(&mut EventIndex::default(), &backward),
        ["25.10.2026 02:45 — 02:15"]
    );
}

#[test]
fn supported_year_boundaries_clip_projection_without_losing_partial_intervals() {
    for properties in [
        "DTSTART:19000101T000000Z\r\n",
        "DTSTART:21001231T233000Z\r\n",
    ] {
        let (year, zone) = if properties.contains("1900") {
            (1900, chrono_tz::America::New_York)
        } else {
            (2100, chrono_tz::Europe::Moscow)
        };
        let document = document(properties, year, DisplayTimeZone::Named(zone));
        assert_eq!(
            labels(&mut EventIndex::default(), &document),
            [] as [String; 0]
        );
    }
    let document = document(
        "DTSTART:19000101T000000Z\r\nDTEND:19000101T060000Z\r\n",
        1900,
        DisplayTimeZone::Named(chrono_tz::America::New_York),
    );
    let mut index = EventIndex::default();
    assert_eq!(
        labels(&mut index, &document),
        ["31.12.1899 19:00 — 01.01.1900 01:00"]
    );
    assert_eq!(index.months[0].days[0].colors().len(), 1);
}

#[test]
fn projecting_into_the_source_zone_resolves_nonexistent_hours_and_dates() {
    for (properties, zone, year, label) in [
        (
            "DTSTART;TZID=Europe/Berlin:20260329T023000\r\n",
            chrono_tz::Europe::Berlin,
            2026,
            "29.03.2026 03:30",
        ),
        (
            "DTSTART;TZID=Pacific/Apia:20111230T090000\r\n",
            chrono_tz::Pacific::Apia,
            2011,
            "31.12.2011 09:00",
        ),
    ] {
        let document = document(properties, year, DisplayTimeZone::Named(zone));
        assert_eq!(labels(&mut EventIndex::default(), &document), [label]);
    }
}
