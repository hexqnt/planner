use chrono::{NaiveDate, NaiveTime};

use super::*;
use crate::model::{DateRange, EventTimeZone, Year};

fn calendar(events: &str) -> String {
    format!("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//Tests//EN\r\n{events}END:VCALENDAR\r\n")
}

fn vevent(uid: &str, properties: &str) -> String {
    format!("BEGIN:VEVENT\r\nUID:{uid}\r\n{properties}END:VEVENT\r\n")
}

fn single(properties: &str) -> Event {
    imported(&calendar(&vevent("test@example.org", properties)))
}

fn imported(input: &str) -> Event {
    let mut events = import(input, CategoryId(1)).unwrap();
    assert_eq!(events.len(), 1);
    events.pop().unwrap()
}

fn starts(event: &Event, start: &str, end: &str) -> Vec<String> {
    event
        .occurrences(DateRange::new(start.parse().unwrap(), end.parse().unwrap()).unwrap())
        .map(|schedule| schedule.dates().start().to_string())
        .collect()
}

fn round_trip(event: Event) -> (String, Event) {
    let original = serde_json::to_value(&event).unwrap();
    let document = Document {
        events: vec![event],
        ..crate::test_support::document()
    };
    let text = export(&document).unwrap();
    let restored = imported(&text);
    assert_eq!(original, serde_json::to_value(&restored).unwrap());
    (text, restored)
}

#[test]
fn all_day_end_is_exclusive_and_year_boundary_round_trips() {
    let event =
        single("SUMMARY:Trip\r\nDTSTART;VALUE=DATE:20261229\r\nDTEND;VALUE=DATE:20270103\r\n");
    assert_eq!(event.schedule.occupied_dates().days(), 5);
    assert_eq!(
        event.schedule.occupied_dates().end().to_string(),
        "2027-01-02"
    );
    let (text, _) = round_trip(event);
    assert!(text.contains("DTEND;VALUE=DATE:20270103"));
    assert!(text.contains("DTSTAMP:"));
    round_trip(single(
        "SUMMARY:Boundary\r\nDTSTART;VALUE=DATE:21001231\r\nDTEND;VALUE=DATE:21010101\r\n",
    ));
}

#[test]
fn absent_summary_and_end_use_standard_defaults() {
    let all_day = single("DTSTART;VALUE=DATE:20261003\r\n");
    assert_eq!(all_day.title.get(), "Untitled event");
    assert_eq!(all_day.schedule.occupied_dates().days(), 1);
    round_trip(all_day);
    assert_eq!(
        single("SUMMARY: \r\nDTSTART;VALUE=DATE:20261003\r\n")
            .title
            .get(),
        "Untitled event"
    );
    for start in [
        "DTSTART:20261003T000000Z",
        "DTSTART:20261003T090000Z",
        "DTSTART:20261003T090000",
        "DTSTART;TZID=Europe/Berlin:20261003T090000",
    ] {
        let event = single(&format!("{start}\r\n"));
        let times = event.schedule.times().unwrap();
        assert_eq!(times.start, times.end);
        assert_eq!(event.schedule.occupied_dates().days(), 1);
        let (text, _) = round_trip(event);
        assert!(!text.contains("DTEND"), "{text}");
    }
}

#[test]
fn text_is_unfolded_and_unescaped_once() {
    let event = single(
        "SUMMARY:Встреча\\, отпуск\\; и \\nследующая строка\r\nDESCRIPTION:line one\\nline two\\\\n\r\nLOCATION:Оф\r\n ис\r\nDTSTART;VALUE=DATE:20261003\r\n",
    );
    assert_eq!(event.title.get(), "Встреча, отпуск; и \nследующая строка");
    assert_eq!(event.notes, "line one\nline two\\n");
    assert_eq!(event.location, "Офис");
    round_trip(event);
}

#[test]
fn weekly_days_count_and_exclusions_are_preserved_and_indexed() {
    let event = single(
        "SUMMARY:Standup\r\nDTSTART;VALUE=DATE:20261005\r\nRRULE:FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,WE;COUNT=5\r\nEXDATE;VALUE=DATE:20261007,20261102\r\nEXDATE;VALUE=DATE:20261007\r\n",
    );
    let expected = ["2026-10-05", "2026-10-19", "2026-10-21"];
    assert_eq!(starts(&event, "2026-10-01", "2026-12-31"), expected);
    assert_eq!(starts(&event, "2026-10-18", "2026-12-31"), &expected[1..]);
    let (_, restored) = round_trip(event);
    let mut document = Document {
        year: Year::try_from(2026).unwrap(),
        ..Document::default()
    };
    document.merge_events(vec![restored]);
    let mut index = crate::model::EventIndex::default();
    let query = |value: &str| {
        let date: NaiveDate = value.parse().unwrap();
        DateRange::between(date, date)
    };
    assert_eq!(index.list(&document, query("2026-10-07")).count(), 0);
    assert_eq!(index.list(&document, query("2026-10-21")).count(), 1);
}

#[test]
fn exclusion_lists_are_normalized_across_repeated_properties() {
    for properties in [
        "DTSTART;VALUE=DATE:20261003\r\nRRULE:FREQ=DAILY;COUNT=4\r\nEXDATE;VALUE=DATE:20261006,20261004,20261006\r\nEXDATE;VALUE=DATE:20261004\r\n",
        "DTSTART:20261003T090000\r\nRRULE:FREQ=DAILY;COUNT=4\r\nEXDATE:20261006T090000,20261004T090000,20261006T090000\r\nEXDATE:20261004T090000\r\n",
        "DTSTART;TZID=Europe/Moscow:20261003T090000\r\nRRULE:FREQ=DAILY;COUNT=4\r\nEXDATE;TZID=Europe/Moscow:20261006T090000,20261004T090000\r\nEXDATE:20261004T060000Z,20261006T060000Z\r\n",
    ] {
        let event = single(properties);
        assert_eq!(event.identity.exclusions.iter().count(), 2);
        assert_eq!(
            starts(&event, "2026-10-01", "2026-10-31"),
            ["2026-10-03", "2026-10-05"]
        );
        round_trip(event);
    }
}

#[test]
fn week_start_changes_interval_alignment() {
    for (wkst, expected) in [
        (
            "MO",
            vec!["2026-10-04", "2026-10-12", "2026-10-18", "2026-10-26"],
        ),
        (
            "SU",
            vec!["2026-10-04", "2026-10-05", "2026-10-18", "2026-10-19"],
        ),
    ] {
        let event = single(&format!(
            "DTSTART;VALUE=DATE:20261004\r\nRRULE:FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,SU;COUNT=4;WKST={wkst}\r\n"
        ));
        assert_eq!(starts(&event, "2026-10-01", "2026-11-01"), expected);
        round_trip(event);
    }
}

#[test]
fn monthly_and_yearly_counts_skip_invalid_dates() {
    let event = single("DTSTART;VALUE=DATE:20260131\r\nRRULE:FREQ=MONTHLY;COUNT=3\r\n");
    assert_eq!(
        starts(&event, "2026-01-01", "2026-12-31"),
        ["2026-01-31", "2026-03-31", "2026-05-31"]
    );
    assert_eq!(starts(&event, "2026-05-01", "2026-12-31"), ["2026-05-31"]);
    round_trip(event);
    let event = single("DTSTART;VALUE=DATE:20240229\r\nRRULE:FREQ=YEARLY;COUNT=3\r\n");
    assert_eq!(
        starts(&event, "2024-01-01", "2040-12-31"),
        ["2024-02-29", "2028-02-29", "2032-02-29"]
    );
    round_trip(event);
}

#[test]
fn until_is_inclusive_and_preserves_exact_time() {
    for (until, expected) in [("085959", 2), ("090000", 3)] {
        let event = single(&format!(
            "DTSTART:20261003T090000Z\r\nDTEND:20261003T100000Z\r\nRRULE:FREQ=DAILY;UNTIL=20261005T{until}Z\r\n"
        ));
        assert_eq!(starts(&event, "2026-10-01", "2026-10-31").len(), expected);
        round_trip(event);
    }
    round_trip(single(
        "DTSTART:20261003T090000\r\nRRULE:FREQ=DAILY;UNTIL=20261005T090000\r\n",
    ));
    round_trip(single(
        "DTSTART;VALUE=DATE:20261003\r\nRRULE:FREQ=DAILY;UNTIL=20261005\r\n",
    ));
}

#[test]
fn timezone_recurring_wall_clocks_survive_dst_and_utc_exclusions() {
    let event = single(
        "DTSTART;TZID=Europe/Berlin:20260327T090000\r\nDTEND;TZID=Europe/Berlin:20260327T100000\r\nRRULE:FREQ=DAILY;COUNT=4\r\nEXDATE:20260329T070000Z\r\n",
    );
    assert_eq!(
        event.schedule.timezone(),
        EventTimeZone::Named(chrono_tz::Europe::Berlin)
    );
    assert_eq!(
        starts(&event, "2026-03-01", "2026-04-01"),
        ["2026-03-27", "2026-03-28", "2026-03-30"]
    );
    let schedules: Vec<_> = event
        .occurrences(Year::try_from(2026).unwrap().range())
        .collect();
    assert!(schedules.iter().all(
        |schedule| schedule.times().unwrap().start == NaiveTime::from_hms_opt(9, 0, 0).unwrap()
    ));
    let zone = event.schedule.timezone();
    let utc = |schedule: crate::model::EventSchedule| {
        zone.to_utc(
            schedule
                .dates()
                .start()
                .and_time(schedule.times().unwrap().start),
        )
        .unwrap()
        .format("%H:%M")
        .to_string()
    };
    assert_eq!(utc(schedules[0]), "08:00");
    assert_eq!(utc(schedules[2]), "07:00");
    let (text, _) = round_trip(event);
    assert!(text.contains("BEGIN:VTIMEZONE"));
    assert!(text.contains("TZOFFSETFROM:+0100\r\nTZOFFSETTO:+0200"));
    round_trip(single(
        "DTSTART;TZID=\"America/New_York\":20261003T230000\r\nDTEND:20261004T040000Z\r\n",
    ));
}

#[test]
fn nonexistent_hours_do_not_consume_count_and_ambiguous_hours_choose_first() {
    let event = single(
        "DTSTART;TZID=Europe/Berlin:20260328T023000\r\nDTEND;TZID=Europe/Berlin:20260328T033000\r\nRRULE:FREQ=DAILY;COUNT=3\r\n",
    );
    assert_eq!(
        starts(&event, "2026-03-01", "2026-04-02"),
        ["2026-03-28", "2026-03-30", "2026-03-31"]
    );
    let zone = EventTimeZone::Named(chrono_tz::Europe::Berlin);
    let time = |value: &str| zone.to_utc(value.parse().unwrap()).unwrap().to_rfc3339();
    assert_eq!(time("2026-10-25T02:30:00"), "2026-10-25T00:30:00+00:00");
    assert_eq!(time("2026-03-29T02:30:00"), "2026-03-29T01:30:00+00:00");
}

#[test]
fn invalid_and_unsupported_schedules_fail_atomically() {
    let valid = vevent("valid", "DTSTART;VALUE=DATE:20261003\r\n");
    for invalid in [
        "DTSTART;VALUE=DATE:20261003\r\nRRULE:FREQ=DAILY;COUNT=0\r\n",
        "DTSTART;VALUE=DATE:20261003\r\nRRULE:FREQ=DAILY;COUNT=2;UNTIL=20261005\r\n",
        "DTSTART;VALUE=DATE:20261003\r\nRRULE:FREQ=MONTHLY;BYDAY=1MO\r\n",
        "DTSTART;VALUE=DATE:20261003\r\nRRULE:FREQ=WEEKLY;BYDAY=MO\r\n",
        "DTSTART;VALUE=DATE:20261003\r\nRRULE:FREQ=DAILY;BYMONTHDAY=3\r\n",
        "DTSTART;VALUE=DATE:20261003\r\nRRULE:FREQ=DAILY;FREQ=WEEKLY\r\n",
        "DTSTART;VALUE=DATE:20261003\r\nDTEND;VALUE=DATE:20261003\r\n",
        "DTSTART:20261003T100000Z\r\nDTEND:20261003T090000Z\r\n",
        "DTSTART;TZID=Europe/Berlin:20261025T023000\r\nDTEND:20261025T014500Z\r\n",
        "DTSTART;TZID=Custom:20261003T100000\r\n",
        "DTSTART;TZID=Europe/Berlin:20261003T100000Z\r\n",
        "DTSTART:20261003T100000\r\nEXDATE;VALUE=DATE:20261003\r\n",
        "DTSTART;VALUE=DATE:20261003\r\nDURATION:P1D\r\n",
        "DTSTART;VALUE=DATE:20261003\r\nRECURRENCE-ID;VALUE=DATE:20261003\r\n",
        "DTSTART;VALUE=DATE:20261003\r\nRDATE;VALUE=DATE:20261004\r\n",
        "DTSTART;VALUE=DATE:18991231\r\n",
        "DTSTART;VALUE=DATE:20260230\r\n",
        "DTSTART;VALUE=DATE:20261003\r\nDTSTART;VALUE=DATE:20261004\r\n",
        "DTSTART;VALUE=DATE:20261003\r\nEXDATE;VALUE=DATE:20261003,broken\r\n",
        "DTSTART;VALUE=DATE:20261003\r\nEXDATE;VALUE=DATE;VALUE=DATE:20261004,20261005\r\n",
        "DTSTART;TZID=Europe/Moscow:20261003T090000\r\nEXDATE;TZID=Europe/Moscow;TZID=UTC:20261004T090000,20261005T090000\r\n",
        "DTSTART:20261003T090000Z\r\nEXDATE;TZID=Europe/Moscow:20261004T060000Z,20261005T060000Z\r\n",
        "DTSTART;VALUE=DATE:20261003\r\nSTATUS:UNKNOWN\r\n",
        "DTSTART;VALUE=DATE:20261003\r\nTRANSP:UNKNOWN\r\n",
        "DTSTART;VALUE=DATE:20261003\r\nURL:example.com\r\n",
        "SUMMARY:Missing start\r\n",
        "DTSTART;VALUE=DATE;TZID=Europe/Moscow:20261003\r\n",
        "DTSTART;VALUE=PERIOD:20261003\r\n",
        "DTSTART;VALUE:20261003\r\n",
        "DTSTART;VALUE=DATE;value=DATE:20261003\r\n",
        "DTSTART:2026-10-03T09:00:00Z\r\n",
        "DTSTART:20261003T090000+0300\r\n",
        "DTSTART;VALUE=DATE:20261003\r\nDTEND:20261004T090000Z\r\n",
        "DTSTART:20261003T090000\r\nDTEND:20261003T100000Z\r\n",
        "DTSTART:20261003T090000Z\r\nDTEND:20261003T100000\r\n",
        "DTSTART;VALUE=DATE:20261003\r\nRRULE:COUNT=2\r\n",
        "DTSTART;VALUE=DATE:20261003\r\nRRULE:FREQ\r\n",
        "DTSTART;VALUE=DATE:20261003\r\nRRULE:FREQ=HOURLY\r\n",
        "DTSTART;VALUE=DATE:20261003\r\nRRULE:FREQ=DAILY;INTERVAL=0\r\n",
        "DTSTART;VALUE=DATE:20261003\r\nRRULE:FREQ=DAILY;WKST=XX\r\n",
        "DTSTART:20261003T090000Z\r\nRRULE:FREQ=DAILY;UNTIL=20261005T090000\r\n",
        "DTSTART:20261003T090000\r\nRRULE:FREQ=DAILY;UNTIL=20261005T090000Z\r\n",
        "DTSTART;VALUE=DATE:20261003\r\nEXDATE:20261004T090000\r\n",
    ] {
        let invalid_event = vevent("invalid", invalid);
        let error = import(&calendar(&format!("{valid}{invalid_event}")), CategoryId(1))
            .err()
            .expect("Invalid event must reject the entire import");
        assert!(
            error.to_string().starts_with("VEVENT 2:"),
            "{invalid}: {error}"
        );
    }
}

#[test]
fn event_uids_are_required_and_unique_within_a_calendar() {
    let event = vevent("event", "DTSTART;VALUE=DATE:20261003\r\n");
    for (events, reason) in [
        (event.replace("UID:event\r\n", ""), "VEVENT 1: Missing UID"),
        (format!("{event}{event}"), "VEVENT 2: Duplicate UID"),
    ] {
        let error = import(&calendar(&events), CategoryId(1)).err().unwrap();
        assert!(error.to_string().starts_with(reason), "{error}");
    }
}

#[test]
fn singleton_properties_reject_duplicates_instead_of_choosing_a_value() {
    for property in [
        "SUMMARY:Meeting",
        "DTSTART;VALUE=DATE:20261003",
        "DTEND;VALUE=DATE:20261004",
        "RRULE:FREQ=DAILY",
        "STATUS:CONFIRMED",
        "TRANSP:OPAQUE",
        "URL:https://example.com/",
    ] {
        let prefix = if property.starts_with("DTSTART") {
            ""
        } else {
            "DTSTART;VALUE=DATE:20261003\r\n"
        };
        let text = calendar(&vevent(
            "event",
            &format!("{prefix}{property}\r\n{property}\r\n"),
        ));
        let error = import(&text, CategoryId(1)).err().unwrap();
        let name = property.split([';', ':']).next().unwrap();
        assert_eq!(error.to_string(), format!("VEVENT 1: Duplicate {name}"));
    }
}

#[test]
fn calendar_structure_is_consumed_completely() {
    let valid = calendar(&vevent("event", "DTSTART;VALUE=DATE:20261003\r\n"));
    for input in [
        format!("{valid}{valid}"),
        format!("{valid}BEGIN:VTODO\r\nEND:VTODO\r\n"),
        format!("{valid}garbage\r\n"),
        valid.replace("END:VCALENDAR\r\n", ""),
        valid.replace("END:VEVENT", "END:VTODO"),
        valid.replace("VCALENDAR", "VTODO"),
        valid.replace("VERSION:2.0", "VERSION:1.0"),
        valid.replace("VERSION:2.0", "VERSION:2.0\r\nMETHOD:CANCEL"),
    ] {
        assert!(import(&input, CategoryId(1)).is_err(), "{input}");
    }
    assert_eq!(
        import(&format!("\u{feff}{valid}"), CategoryId(1))
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn non_events_are_ignored() {
    let input = calendar(
        "BEGIN:VTODO\r\nUID:todo\r\nEND:VTODO\r\nBEGIN:VEVENT\r\nUID:event\r\nDTSTART;VALUE=DATE:20261003\r\nEND:VEVENT\r\n",
    );
    let events = import(&input, CategoryId(1)).unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].identity.uid.as_str(), "event");
}

#[test]
fn optional_details_parse_to_the_expected_values_and_round_trip() {
    for (property, expected) in [
        ("", None),
        ("STATUS:TENTATIVE\r\n", Some(EventStatus::Tentative)),
        ("STATUS:CONFIRMED\r\n", Some(EventStatus::Confirmed)),
        ("STATUS:CANCELLED\r\n", Some(EventStatus::Cancelled)),
    ] {
        let event = single(&format!("DTSTART;VALUE=DATE:20261003\r\n{property}"));
        assert_eq!(event.status, expected);
        round_trip(event);
    }
    for (property, expected) in [
        ("", Availability::Busy),
        ("TRANSP:OPAQUE\r\n", Availability::Busy),
        ("TRANSP:TRANSPARENT\r\n", Availability::Free),
    ] {
        let event = single(&format!(
            "DTSTART;VALUE=DATE:20261003\r\n{property}URL:https://example.com/\r\n"
        ));
        assert_eq!(event.availability, expected);
        assert_eq!(
            event.link.as_ref().unwrap().as_str(),
            "https://example.com/"
        );
        round_trip(event);
    }
}

#[test]
fn calendars_without_events_report_an_empty_import() {
    for components in ["", "BEGIN:VTODO\r\nUID:todo\r\nEND:VTODO\r\n"] {
        let error = import(&calendar(components), CategoryId(1)).err().unwrap();
        assert_eq!(error.to_string(), "No VEVENT events found");
    }
}

#[test]
fn utc_exclusions_and_until_distinguish_both_occurrences_of_an_ambiguous_hour() {
    for (excluded, remaining) in [("003000", 0), ("013000", 1)] {
        let event = single(&format!(
            "DTSTART;TZID=Europe/Berlin:20261025T023000\r\nEXDATE:20261025T{excluded}Z\r\n"
        ));
        assert_eq!(starts(&event, "2026-10-25", "2026-10-25").len(), remaining);
        round_trip(event);
    }
    let event = single(
        "DTSTART;TZID=Europe/Berlin:20261024T024500\r\nRRULE:FREQ=DAILY;UNTIL=20261025T011500Z\r\n",
    );
    assert_eq!(
        starts(&event, "2026-10-01", "2026-10-31"),
        ["2026-10-24", "2026-10-25"]
    );
    let (text, _) = round_trip(event);
    assert!(text.contains("UNTIL=20261025T011500Z"));
    round_trip(single(
        "DTSTART:20261003T090000Z\r\nDTEND;TZID=Europe/Berlin:20261003T120000\r\n",
    ));
}

#[test]
fn property_names_and_enumerated_values_are_case_insensitive() {
    let input = "begin:vcalendar\r\nVersion:2.0\r\nmethod:publish\r\nBEGIN:vevent\r\nUid:MixedCase@example.org\r\nsummary:Встреча\\, офис\\; этаж 2\r\ndescription;value=text:Line\\Nnext\\\\n\r\nlocation;altrep=\"https://example.org/Room;a:b\":Room A\\, B\r\nDtStart;TzId=\"Europe/Berlin\";Value=date-time:20261005T090000\r\nDtEnd;tzid=Europe/Berlin:20261005T100000\r\nrrule:freq=weekly;interval=2;byday=mo,we;count=4;wkst=su\r\nexdate;tzid=Europe/Berlin:20261007T090000\r\nstatus:confirmed\r\ntransp:transparent\r\nurl:https://example.org/MixedCase?a=b,c;d=e\r\nEND:VEVENT\r\nend:VCALENDAR\r\n";
    let event = imported(input);
    assert_eq!(event.identity.uid.as_str(), "MixedCase@example.org");
    assert_eq!(event.title.get(), "Встреча, офис; этаж 2");
    assert_eq!(event.notes, "Line\nnext\\n");
    assert_eq!(event.location, "Room A, B");
    assert_eq!(event.status, Some(EventStatus::Confirmed));
    assert_eq!(event.availability, Availability::Free);
    assert_eq!(
        event.link.as_ref().unwrap().as_str(),
        "https://example.org/MixedCase?a=b,c;d=e"
    );
    assert_eq!(
        starts(&event, "2026-10-01", "2026-11-01"),
        ["2026-10-05", "2026-10-19", "2026-10-21"]
    );
    round_trip(event);
    round_trip(single("dtstart;value=date:20261003\r\n"));
}

#[test]
fn export_has_exactly_one_product_id_and_version_even_without_events() {
    for events in [Vec::new(), vec![single("DTSTART;VALUE=DATE:20261003\r\n")]] {
        let document = Document {
            events,
            ..crate::test_support::document()
        };
        let text = export(&document).unwrap();
        let unfolded = icalendar::parser::unfold(&text);
        let calendars = icalendar::parser::read_components(&unfolded).unwrap();
        let [calendar] = calendars.as_slice() else {
            panic!("Expected one calendar");
        };
        for (name, expected) in [
            ("VERSION", "2.0"),
            ("PRODID", "-//planner//Year planner//EN"),
            ("CALSCALE", "GREGORIAN"),
        ] {
            assert_eq!(required(calendar, name).unwrap().val.as_str(), expected);
        }
    }
}

#[test]
fn unicode_text_is_folded_by_octets_and_preserved_without_touching_uris() {
    let mut event = single("DTSTART;VALUE=DATE:20261003\r\n");
    event.title = "Встреча 🗓️; обсуждение, планы ".repeat(5).parse().unwrap();
    event.notes = format!("{}\nC:\\next\\node, конец; :\\", "Строка 🦀 ".repeat(30));
    event.location = "Офис, этаж; зал: №1".into();
    event.identity.uid =
        EventUid::try_from("MixedCase,semi;slash\\uid@example.org".to_owned()).unwrap();
    event.link = Some("https://example.org/MixedCase?a=b,c;d=e".parse().unwrap());
    let (text, _) = round_trip(event);
    assert!(text.contains("\r\n "));
    assert!(text.ends_with("\r\n"));
    for line in text.split_inclusive('\n') {
        let content = line.strip_suffix("\r\n").expect("CRLF line ending");
        assert!(content.len() <= 75, "{} bytes: {content}", content.len());
    }
    assert!(text.contains("URL:https://example.org/MixedCase?a=b,c;d=e\r\n"));
    assert!(text.contains("UID:MixedCase\\,semi\\;slash\\\\uid@example.org\r\n"));
}

#[test]
fn timezone_definitions_are_unique_and_contain_expected_offsets() {
    let mut events = Vec::new();
    for (index, zone) in [
        "America/St_Johns",
        "Asia/Kathmandu",
        "Australia/Lord_Howe",
        "Australia/Lord_Howe",
    ]
    .into_iter()
    .enumerate()
    {
        let mut event = single(&format!(
            "DTSTART;TZID={zone}:20261003T090000\r\nDTEND;TZID={zone}:20261003T100000\r\n"
        ));
        event.identity.uid = EventUid::try_from(format!("zone-{index}@tests")).unwrap();
        events.push(event);
    }
    let document = Document {
        events,
        ..crate::test_support::document()
    };
    let text = export(&document).unwrap();
    let unfolded = icalendar::parser::unfold(&text);
    let calendars = icalendar::parser::read_components(&unfolded).unwrap();
    let zones: Vec<_> = calendars[0]
        .components
        .iter()
        .filter(|component| component.name == "VTIMEZONE")
        .collect();
    assert_eq!(zones.len(), 3);
    for (zone, expected_id) in
        zones
            .iter()
            .zip(["America/St_Johns", "Asia/Kathmandu", "Australia/Lord_Howe"])
    {
        assert_eq!(required(zone, "TZID").unwrap().val.as_str(), expected_id);
        assert!(property(zone, "UID").unwrap().is_none());
        assert!(property(zone, "DTSTAMP").unwrap().is_none());
        assert_ne!(zone.components, [] as [Component<'_>; 0]);
        for observance in &zone.components {
            assert!(matches!(observance.name.as_str(), "STANDARD" | "DAYLIGHT"));
            assert!(
                !required(observance, "DTSTART")
                    .unwrap()
                    .val
                    .as_str()
                    .ends_with('Z')
            );
            required(observance, "TZOFFSETFROM").unwrap();
            required(observance, "TZOFFSETTO").unwrap();
        }
    }
    assert!(text.contains("TZOFFSETTO:-0330\r\n"));
    assert!(text.contains("TZOFFSETTO:+0545\r\n"));
    assert!(text.contains("TZOFFSETFROM:+1030\r\nTZOFFSETTO:+1100\r\n"));
    assert_eq!(import(&text, CategoryId(1)).unwrap().len(), 4);
}
