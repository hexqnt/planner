use super::*;
use crate::{
    model::EventTimes,
    test_support::{date, range},
};

fn starts(start: &str, frequency: Frequency, interval: u16, end: &str) -> Vec<String> {
    let date: NaiveDate = start.parse().unwrap();
    let schedule = EventSchedule::all_day(DateRange::between(date, date))
        .with_recurrence(Some(
            Recurrence::new(frequency, NonZeroU16::new(interval).unwrap(), None).unwrap(),
        ))
        .unwrap();
    schedule
        .occurrences(DateRange::between(date, end.parse().unwrap()))
        .map(|occurrence| occurrence.dates().start().to_string())
        .collect()
}

#[test]
fn changing_a_cutoff_drops_the_previous_absolute_boundary() {
    let date = "2026-10-03".parse().unwrap();
    let rule = Recurrence::new(Frequency::Daily, NonZeroU16::MIN, Some(date))
        .unwrap()
        .with_pattern(None, None, Weekday::Mon, Some("09:00".parse().unwrap()))
        .unwrap()
        .with_until_instant(Some("2026-10-03T07:00:00Z".parse().unwrap()))
        .unwrap();
    let changed = rule
        .with_pattern(None, None, Weekday::Mon, Some("10:00".parse().unwrap()))
        .unwrap();
    assert_eq!(changed.until_time(), Some("10:00".parse().unwrap()));
    assert_eq!(changed.until_instant(), None);
    let local = rule.with_until_instant(None).unwrap();
    assert_eq!(local.until(), rule.until());
    assert_eq!(local.until_time(), rule.until_time());
    assert_eq!(local.until_instant(), None);
}

#[test]
fn weekday_masks_accept_exactly_nonempty_sets_of_the_seven_weekdays() {
    let days = [
        Weekday::Mon,
        Weekday::Tue,
        Weekday::Wed,
        Weekday::Thu,
        Weekday::Fri,
        Weekday::Sat,
        Weekday::Sun,
    ];
    for bits in 0..=u8::MAX {
        let parsed = Weekdays::try_from(bits);
        let restored = serde_json::from_value::<Weekdays>(serde_json::json!(bits));
        if (1..=0x7f).contains(&bits) {
            let mask = parsed.unwrap();
            assert_eq!(restored.unwrap(), mask);
            assert_eq!(u8::from(mask), bits);
            for (index, day) in days.into_iter().enumerate() {
                assert_eq!(
                    mask.contains(day),
                    bits & (1 << index) != 0,
                    "{bits:#x}, {day:?}"
                );
            }
        } else {
            assert_eq!(parsed, Err(InputError::RecurrencePattern));
            assert!(restored.is_err(), "{bits:#x}");
        }
    }
}

#[test]
fn recurrence_patterns_reject_conflicting_endings_and_inapplicable_weekdays() {
    let until = date("2026-10-05");
    let time = "09:00".parse().unwrap();
    let weekdays = Some(Weekdays::try_from(0x7f).unwrap());
    for (frequency, until, count, weekdays, until_time) in [
        (Frequency::Daily, None, None, weekdays, None),
        (Frequency::Monthly, None, None, weekdays, None),
        (Frequency::Yearly, None, None, weekdays, None),
        (
            Frequency::Weekly,
            Some(until),
            NonZeroU32::new(2),
            weekdays,
            None,
        ),
        (Frequency::Daily, None, None, None, Some(time)),
        (Frequency::Daily, None, NonZeroU32::new(2), None, Some(time)),
    ] {
        let rule = Recurrence::new(frequency, NonZeroU16::MIN, until).unwrap();
        assert_eq!(
            rule.with_pattern(count, weekdays, Weekday::Mon, until_time),
            Err(InputError::RecurrencePattern)
        );
    }
    let unlimited = Recurrence::new(Frequency::Daily, NonZeroU16::MIN, None).unwrap();
    assert_eq!(
        unlimited.with_until_instant(Some("2026-10-05T09:00:00Z".parse().unwrap())),
        Err(InputError::RecurrencePattern)
    );
}

#[test]
fn deserialization_checks_recurrence_invariants_and_legacy_defaults() {
    let legacy = serde_json::json!({"frequency": "Weekly", "interval": 2, "until": null});
    let rule: Recurrence = serde_json::from_value(legacy.clone()).unwrap();
    assert_eq!(rule.frequency(), Frequency::Weekly);
    assert_eq!(rule.interval().get(), 2);
    assert_eq!(rule.count(), None);
    assert_eq!(rule.weekdays(), None);
    assert_eq!(rule.week_start(), Weekday::Mon);
    for patch in [
        serde_json::json!({"interval": 0}),
        serde_json::json!({"count": 0}),
        serde_json::json!({"weekdays": 0}),
        serde_json::json!({"weekdays": 128}),
        serde_json::json!({"frequency": "Daily", "weekdays": 1}),
        serde_json::json!({"count": 2, "until": "2026-10-05"}),
        serde_json::json!({"until_time": "09:00:00"}),
        serde_json::json!({"until_instant": "2026-10-05T09:00:00Z"}),
        serde_json::json!({"until": "1899-12-31"}),
        serde_json::json!({"until": "2101-01-01"}),
    ] {
        let mut invalid = legacy.clone();
        invalid
            .as_object_mut()
            .unwrap()
            .extend(patch.as_object().unwrap().clone());
        assert!(
            serde_json::from_value::<Recurrence>(invalid).is_err(),
            "{patch}"
        );
    }
}

#[test]
fn schedule_rejects_cutoffs_before_start_and_weekdays_without_the_anchor() {
    let dates = range("2026-10-05", "2026-10-05");
    let schedule = EventSchedule::timed(
        dates,
        EventTimes {
            start: "09:00".parse().unwrap(),
            end: "10:00".parse().unwrap(),
        },
    )
    .unwrap();
    let before =
        Recurrence::new(Frequency::Daily, NonZeroU16::MIN, Some(date("2026-10-04"))).unwrap();
    assert_eq!(
        schedule.with_recurrence(Some(before)),
        Err(InputError::RecurrenceUntil)
    );
    let on_start_day =
        Recurrence::new(Frequency::Daily, NonZeroU16::MIN, Some(dates.start())).unwrap();
    for (time, expected) in [
        ("08:59:59", Err(InputError::RecurrenceUntil)),
        ("09:00:00", Ok(())),
    ] {
        let rule = on_start_day
            .with_pattern(None, None, Weekday::Mon, Some(time.parse().unwrap()))
            .unwrap();
        assert_eq!(schedule.with_recurrence(Some(rule)).map(|_| ()), expected);
        assert_eq!(
            EventSchedule::all_day(dates).with_recurrence(Some(rule)),
            Err(InputError::RecurrencePattern)
        );
    }
    let rule = Recurrence::new(Frequency::Weekly, NonZeroU16::MIN, None)
        .unwrap()
        .with_pattern(
            None,
            Some(Weekdays::try_from(1 << 1).unwrap()),
            Weekday::Mon,
            None,
        )
        .unwrap();
    assert_eq!(
        schedule.with_recurrence(Some(rule)),
        Err(InputError::RecurrencePattern)
    );
    assert_eq!(schedule.with_recurrence(None).unwrap(), schedule);
}

#[test]
fn overlapping_instances_respect_until_and_exclusive_midnight() {
    let schedule = EventSchedule::timed(
        DateRange::between("2026-12-30".parse().unwrap(), "2027-01-02".parse().unwrap()),
        crate::model::EventTimes {
            start: "12:00:00".parse().unwrap(),
            end: chrono::NaiveTime::MIN,
        },
    )
    .unwrap()
    .with_recurrence(Some(
        Recurrence::new(
            Frequency::Weekly,
            NonZeroU16::MIN,
            Some("2027-01-06".parse().unwrap()),
        )
        .unwrap(),
    ))
    .unwrap();
    let day = |value: &str| {
        let date = value.parse().unwrap();
        DateRange::between(date, date)
    };
    let occurrences: Vec<_> = schedule.occurrences(day("2027-01-01")).collect();
    assert_eq!(occurrences.len(), 1);
    assert_eq!(occurrences[0].dates().start(), schedule.dates().start());
    assert_eq!(schedule.occurrences(day("2027-01-02")).count(), 0);
    assert_eq!(schedule.occurrences(day("2027-01-08")).count(), 1);
    assert_eq!(schedule.occurrences(day("2027-01-09")).count(), 0);
    assert_eq!(schedule.occurrences(day("2027-01-13")).count(), 0);
}

#[test]
fn one_off_schedules_yield_only_overlapping_occupied_dates() {
    let dates = range("2026-10-03", "2026-10-04");
    for (schedule, last_day) in [
        (EventSchedule::all_day(dates), "2026-10-04"),
        (
            EventSchedule::timed(
                dates,
                EventTimes {
                    start: "23:00".parse().unwrap(),
                    end: NaiveTime::MIN,
                },
            )
            .unwrap(),
            "2026-10-03",
        ),
        (
            EventSchedule::timed(
                dates,
                EventTimes {
                    start: "23:00".parse().unwrap(),
                    end: "00:00:01".parse().unwrap(),
                },
            )
            .unwrap(),
            "2026-10-04",
        ),
    ] {
        for (day, expected) in [
            ("2026-10-02", false),
            ("2026-10-03", true),
            ("2026-10-04", last_day == "2026-10-04"),
            ("2026-10-05", false),
        ] {
            let occurrences: Vec<_> = schedule.occurrences(range(day, day)).collect();
            let expected = if expected {
                std::slice::from_ref(&schedule)
            } else {
                &[]
            };
            assert_eq!(occurrences, expected, "{schedule:?}, query {day}");
        }
        assert_eq!(
            schedule
                .occurrences(range("2026-01-01", "2026-12-31"))
                .collect::<Vec<_>>(),
            [schedule]
        );
    }
}

fn assert_seeking_matches_full_series(schedule: EventSchedule, full: DateRange) {
    let series: Vec<_> = schedule.occurrences(full).collect();
    assert_eq!(
        series.first().map(|occurrence| occurrence.dates().start()),
        Some(schedule.dates().start()),
        "The full query must include the anchor: {schedule:?}"
    );
    assert!(
        series
            .iter()
            .all(|occurrence| occurrence.recurrence().is_none())
    );
    for start in full
        .start()
        .iter_days()
        .step_by(23)
        .take_while(|date| *date <= full.end())
    {
        let query = DateRange::between(start, (start + chrono::Duration::days(17)).min(full.end()));
        let expected: Vec<_> = series
            .iter()
            .copied()
            .filter(|event| event.occupied_dates().overlaps(query))
            .collect();
        assert_eq!(
            schedule.occurrences(query).collect::<Vec<_>>(),
            expected,
            "{schedule:?}, query {query:?}"
        );
    }
}

#[test]
fn seeking_matches_filtering_full_series_with_all_frequencies_and_endings() {
    let full = range("2024-01-01", "2028-12-31");
    for frequency in [
        Frequency::Daily,
        Frequency::Weekly,
        Frequency::Monthly,
        Frequency::Yearly,
    ] {
        for interval in [1, 2, 13] {
            for anchor in ["2024-01-31", "2024-02-29"] {
                let start = date(anchor);
                let schedule = EventSchedule::all_day(DateRange::between(
                    start,
                    start + chrono::Duration::days(40),
                ));
                let unlimited =
                    Recurrence::new(frequency, NonZeroU16::new(interval).unwrap(), None).unwrap();
                let until =
                    Recurrence::new(frequency, unlimited.interval(), Some(date("2027-03-01")))
                        .unwrap();
                let counted = unlimited
                    .with_pattern(NonZeroU32::new(7), None, Weekday::Mon, None)
                    .unwrap();
                for rule in [unlimited, until, counted] {
                    assert_seeking_matches_full_series(
                        schedule.with_recurrence(Some(rule)).unwrap(),
                        full,
                    );
                }
            }
        }
    }
}

#[test]
fn weekly_patterns_seek_correctly_for_every_week_start_and_end_condition() {
    let full = range("2026-10-01", "2027-02-01");
    let schedule = EventSchedule::all_day(range("2026-10-04", "2026-10-06"));
    for week_start in [
        Weekday::Mon,
        Weekday::Tue,
        Weekday::Wed,
        Weekday::Thu,
        Weekday::Fri,
        Weekday::Sat,
        Weekday::Sun,
    ] {
        for bits in [1 << 6, 0x41, 0x7f] {
            let days = Some(Weekdays::try_from(bits).unwrap());
            let unlimited =
                Recurrence::new(Frequency::Weekly, NonZeroU16::new(2).unwrap(), None).unwrap();
            let until = Recurrence::new(
                Frequency::Weekly,
                unlimited.interval(),
                Some(date("2026-11-15")),
            )
            .unwrap();
            for (rule, count) in [
                (unlimited, None),
                (until, None),
                (unlimited, NonZeroU32::new(5)),
            ] {
                let rule = rule.with_pattern(count, days, week_start, None).unwrap();
                assert_seeking_matches_full_series(
                    schedule.with_recurrence(Some(rule)).unwrap(),
                    full,
                );
            }
        }
    }
}

#[test]
fn counted_series_skip_invalid_dates_without_restarting_the_count_for_a_late_query() {
    let schedule = EventSchedule::all_day(range("2026-01-31", "2026-02-02"))
        .with_recurrence(Some(
            Recurrence::new(Frequency::Monthly, NonZeroU16::MIN, None)
                .unwrap()
                .with_pattern(NonZeroU32::new(3), None, Weekday::Mon, None)
                .unwrap(),
        ))
        .unwrap();
    let full: Vec<_> = schedule
        .occurrences(range("2026-01-01", "2026-12-31"))
        .map(EventSchedule::dates)
        .collect();
    assert_eq!(
        full,
        [
            range("2026-01-31", "2026-02-02"),
            range("2026-03-31", "2026-04-02"),
            range("2026-05-31", "2026-06-02")
        ]
    );
    assert_eq!(
        schedule
            .occurrences(range("2026-06-02", "2026-06-02"))
            .map(EventSchedule::dates)
            .collect::<Vec<_>>(),
        &full[2..]
    );
    assert_eq!(
        schedule
            .occurrences(range("2026-06-03", "2026-12-31"))
            .count(),
        0
    );
}

#[test]
fn preserves_anchor_and_skips_invalid_calendar_dates() {
    assert_eq!(
        starts("2026-01-31", Frequency::Monthly, 1, "2026-05-31"),
        ["2026-01-31", "2026-03-31", "2026-05-31"]
    );
    assert_eq!(
        starts("2024-02-29", Frequency::Yearly, 1, "2032-03-01"),
        ["2024-02-29", "2028-02-29", "2032-02-29"]
    );
    assert_eq!(
        starts("2026-12-25", Frequency::Weekly, 2, "2027-01-31"),
        ["2026-12-25", "2027-01-08", "2027-01-22"]
    );
}
