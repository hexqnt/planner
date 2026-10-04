use super::*;
use crate::{
    model::{EventSchedule, Frequency, Recurrence},
    test_support::{document, event, range},
};

#[test]
fn long_events_fill_only_existing_days_in_every_supported_year() {
    let mut document = document();
    document.events.push(event(
        1,
        CategoryId(1),
        EventSchedule::all_day(range("1900-01-01", "2100-12-31")),
    ));
    let color = document
        .category(document.events[0].category)
        .unwrap()
        .color;
    let mut index = EventIndex::default();
    for year in 1900..=2100 {
        document.year = Year::try_from(year).unwrap();
        index.refresh(&document);
        for (number, month) in (1..=12).zip(&index.months) {
            assert_eq!(month.events.len(), 1);
            for (day, markers) in (1..=31).zip(&month.days) {
                let expected = if chrono::NaiveDate::from_ymd_opt(year, number, day).is_some() {
                    std::slice::from_ref(&color)
                } else {
                    &[]
                };
                assert_eq!(markers.colors(), expected, "{year}-{number}-{day}");
            }
        }
    }
}

#[test]
fn recurring_instances_keep_source_event_and_refresh_visibility() {
    let mut document = document();
    document.year = Year::try_from(2027).unwrap();
    let start = "2026-12-25".parse().unwrap();
    let schedule = EventSchedule::all_day(DateRange::between(start, start))
        .with_recurrence(Some(
            Recurrence::new(
                Frequency::Weekly,
                std::num::NonZeroU16::new(2).unwrap(),
                Some("2027-01-22".parse().unwrap()),
            )
            .unwrap(),
        ))
        .unwrap();
    document.events.push(event(1, CategoryId(1), schedule));
    let mut index = EventIndex::default();
    let rows: Vec<_> = index
        .list(&document, document.year.range())
        .map(|entry| (entry.event.id, entry.occurrence.date_label().to_owned()))
        .collect();
    assert_eq!(
        rows,
        [
            (document.events[0].id, "08.01.2027".to_owned()),
            (document.events[0].id, "22.01.2027".to_owned()),
        ]
    );
    assert_eq!(index.months[0].events.len(), 2);
    assert_eq!(index.months[0].days[7].colors().len(), 1);
    assert!(
        index.months[0]
            .events
            .iter()
            .all(|event| event.source == EventPosition(0))
    );
    document.groups[0].enabled = false;
    index.invalidate();
    assert!(
        index
            .list(&document, document.year.range())
            .all(|entry| !entry.occurrence.visible)
    );
    assert!(index.months[0].events.is_empty());
    let previous = "2026-12-25".parse().unwrap();
    assert_eq!(
        index
            .list(&document, DateRange::between(previous, previous))
            .count(),
        1
    );
}

#[test]
fn occurrence_positions_resolve_to_source_events_after_rebuilding() {
    let mut document = document();
    let start = "2026-01-01".parse().unwrap();
    let end = "2026-01-03".parse().unwrap();
    let query = DateRange::between(start, end);
    let recurring = EventSchedule::all_day(DateRange::between(start, start))
        .with_recurrence(Some(
            Recurrence::new(Frequency::Daily, std::num::NonZeroU16::MIN, Some(end)).unwrap(),
        ))
        .unwrap();
    document.events = vec![
        event(1, CategoryId(1), recurring),
        event(2, CategoryId(7), EventSchedule::all_day(query)),
    ];
    let mut index = EventIndex::default();
    let entries: Vec<_> = index.list(&document, query).collect();
    assert_eq!(entries.len(), 4);
    for (entry, source) in entries.iter().zip([0, 0, 0, 1]) {
        assert!(std::ptr::eq(
            entry.event,
            &raw const document.events[source]
        ));
    }
    assert_eq!(index.months[0].events.len(), 4);
    for (occurrence, source) in index.months[0].events.iter().zip([0, 0, 0, 1]) {
        assert!(std::ptr::eq(
            occurrence.source_event(&document),
            &raw const document.events[source]
        ));
    }
    document.events.remove(0);
    index.invalidate();
    {
        let mut entries = index.list(&document, query);
        assert_eq!(entries.len(), 1);
        assert!(std::ptr::eq(
            entries.next().unwrap().event,
            &raw const document.events[0]
        ));
    }
    assert!(std::ptr::eq(
        index.months[0].events[0].source_event(&document),
        &raw const document.events[0]
    ));
}

#[test]
fn list_cache_tracks_selection_and_rebuilt_document() {
    let mut document = document();
    document.events = vec![
        event(
            1,
            CategoryId(2),
            EventSchedule::all_day(range("2026-01-23", "2026-01-23")),
        ),
        event(
            2,
            CategoryId(7),
            EventSchedule::all_day(range("2026-02-24", "2026-02-28")),
        ),
    ];
    let mut index = EventIndex::default();
    index.refresh(&document);
    let range = DateRange::between("2026-01-23".parse().unwrap(), "2026-01-23".parse().unwrap());
    let _ = index.list(&document, range);
    assert_eq!(index.matching_rows, [OccurrencePosition(0)]);
    assert!(index.rows.iter().all(|row| row.date_label.get().is_none()));
    assert_eq!(index.rows[0].date_label(), "23.01.2026");
    let label = index.rows[0].date_label().as_ptr();
    let _ = index.list(&document, range);
    assert_eq!(index.matching_rows, [OccurrencePosition(0)]);
    assert_eq!(index.rows[0].date_label().as_ptr(), label);
    let _ = index.list(&document, document.year.range());
    assert_eq!(index.matching_rows.len(), document.events.len());
    document.groups[0].enabled = false;
    document.groups[0].categories[1].color = [1, 2, 3];
    index.invalidate();
    let _ = index.list(&document, range);
    assert_eq!(index.matching_rows, [OccurrencePosition(0)]);
    assert!(!index.rows[0].visible);
    assert_eq!(index.rows[0].color, [1, 2, 3]);
    document.events.remove(0);
    index.invalidate();
    let _ = index.list(&document, range);
    assert_eq!(index.matching_rows, [] as [OccurrencePosition; 0]);
}

#[test]
fn month_rows_and_day_markers_clip_both_year_boundaries_and_respect_visibility() {
    let mut document = document();
    document.year = Year::try_from(2024).unwrap();
    document.events = [
        ("2023-12-29", "2024-03-01", CategoryId(1)),
        ("2024-02-29", "2024-02-29", CategoryId(7)),
        ("2024-12-31", "2025-01-03", CategoryId(1)),
        ("2023-12-31", "2023-12-31", CategoryId(1)),
        ("2025-01-01", "2025-01-01", CategoryId(1)),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (start, end, category))| {
        event(
            u64::try_from(index + 1).unwrap(),
            category,
            EventSchedule::all_day(range(start, end)),
        )
    })
    .collect();
    let mut index = EventIndex::default();
    for group_enabled in [false, true] {
        for category_enabled in [false, true] {
            document.groups[0].enabled = group_enabled;
            document.groups[0].categories[0].enabled = category_enabled;
            index.invalidate();
            index.refresh(&document);
            for (number, month) in (1..=12).zip(&index.months) {
                let first = document.year.range().start().with_month(number).unwrap();
                let last = first
                    .with_day(u32::from(first.num_days_in_month()))
                    .unwrap();
                let month_range = DateRange::between(first, last);
                let expected_events: Vec<_> = document
                    .events
                    .iter()
                    .filter(|event| {
                        document.visible_category(event.category).is_some()
                            && event.schedule.occupied_dates().overlaps(month_range)
                    })
                    .collect();
                assert_eq!(month.events.len(), expected_events.len(), "month {number}");
                for (row, expected) in month.events.iter().zip(expected_events) {
                    assert!(std::ptr::eq(row.source_event(&document), expected));
                    assert_eq!(row.range, expected.schedule.occupied_dates());
                }
                for (day, markers) in (1..=31).zip(&month.days) {
                    let date = chrono::NaiveDate::from_ymd_opt(2024, number, day);
                    let expected: Vec<_> = document
                        .events
                        .iter()
                        .filter(|event| {
                            date.is_some_and(|date| event.schedule.occupied_dates().contains(date))
                        })
                        .filter_map(|event| {
                            document
                                .visible_category(event.category)
                                .map(|category| category.color)
                        })
                        .collect();
                    assert_eq!(
                        markers.colors(),
                        expected,
                        "2024-{number}-{day}; group={group_enabled}, category={category_enabled}"
                    );
                }
            }
        }
    }
}

#[test]
fn marker_limit_does_not_truncate_month_rows_or_event_lists() {
    let mut document = document();
    let day = range("2026-01-23", "2026-01-23");
    document.events = document
        .categories()
        .take(8)
        .enumerate()
        .map(|(index, category)| {
            event(
                u64::try_from(index + 1).unwrap(),
                category.id,
                EventSchedule::all_day(day),
            )
        })
        .collect();
    let colors: Vec<_> = document
        .events
        .iter()
        .take(6)
        .map(|event| document.category(event.category).unwrap().color)
        .collect();
    let mut index = EventIndex::default();
    assert_eq!(index.list(&document, day).len(), 8);
    assert_eq!(index.months[0].events.len(), 8);
    assert_eq!(index.months[0].days[22].colors(), colors);
    assert_eq!(index.months[0].days[21].colors(), [] as [[u8; 3]; 0]);
    assert_eq!(index.months[0].days[23].colors(), [] as [[u8; 3]; 0]);
    document.events.clear();
    index.invalidate();
    assert_eq!(index.list(&document, day).len(), 0);
    assert!(index.months.iter().all(
        |month| month.events.is_empty() && month.days.iter().all(|day| day.colors().is_empty())
    ));
}

#[test]
fn queries_on_both_sides_of_the_display_year_expand_the_cache_without_stale_matches() {
    let mut document = document();
    let start = "2025-01-23".parse().unwrap();
    let schedule = EventSchedule::all_day(DateRange::between(start, start))
        .with_recurrence(Some(
            Recurrence::new(Frequency::Yearly, std::num::NonZeroU16::MIN, None).unwrap(),
        ))
        .unwrap();
    document.events.push(event(1, CategoryId(1), schedule));
    let mut index = EventIndex::default();
    for (start, end, expected) in [
        ("2026-01-23", "2026-01-23", vec!["2026-01-23"]),
        ("2025-01-23", "2025-01-23", vec!["2025-01-23"]),
        ("2027-01-23", "2027-01-23", vec!["2027-01-23"]),
        ("2027-01-24", "2027-01-24", vec![]),
        (
            "2025-01-23",
            "2027-01-23",
            vec!["2025-01-23", "2026-01-23", "2027-01-23"],
        ),
        ("2026-01-23", "2026-01-23", vec!["2026-01-23"]),
    ] {
        let query = range(start, end);
        for _ in 0..2 {
            let actual: Vec<_> = index
                .list(&document, query)
                .map(|entry| entry.occurrence.start().to_string())
                .collect();
            assert_eq!(actual, expected, "{query:?}");
            assert_eq!(index.months[0].events.len(), 1);
            assert_eq!(index.months[0].days[22].colors().len(), 1);
        }
    }
}
