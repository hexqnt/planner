use std::num::NonZeroU16;

use super::*;
use crate::{
    model::{
        DisplayTimeZone, EventSchedule, EventTimeZone, EventTimes, Frequency, OccurrenceStart,
        Recurrence, Title,
    },
    test_support::{date, document, event, range},
};

fn search(document: &Document, input: &str, filters: SearchFilters) -> SearchIndex {
    let query = SearchQuery::parse(input).unwrap();
    let mut index = SearchIndex::default();
    index.restart();
    while index.is_pending() {
        index.step(
            document,
            &query,
            filters,
            date("2026-10-03").and_hms_opt(12, 0, 0).unwrap().and_utc(),
        );
    }
    index
}

fn sample() -> Document {
    let mut document = document();
    let category = document.categories().next().unwrap().id;
    let mut meeting = event(
        1,
        category,
        EventSchedule::all_day(range("2026-10-12", "2026-10-14")),
    );
    meeting.title = Title::try_from("Планирование квартала").unwrap();
    meeting.location = "Офис, переговорная 2".into();
    meeting.notes = "Обсудить бюджет и задачи".into();
    meeting
        .details
        .participants
        .push("anna@example.com".parse().unwrap());
    meeting.link = Some("https://example.com/meeting".parse().unwrap());
    meeting
        .details
        .attachments
        .push("https://example.com/report".parse().unwrap());
    document.events.push(meeting);
    document
}

#[test]
fn words_match_across_fields_and_unicode_is_case_insensitive() {
    let document = sample();
    for query in [
        "ПЛАН ОФИС",
        "плн ann",
        "задачи",
        "example.com/meeting",
        "report",
    ] {
        let index = search(&document, query, SearchFilters::default());
        assert_eq!(index.results().len(), 1, "Query: {query}");
        assert_eq!(index.results()[0].event, EventId(1));
    }
    assert!(
        search(&document, "план missing", SearchFilters::default())
            .results()
            .is_empty()
    );
    let index = search(&document, "план офис", SearchFilters::default());
    assert_eq!(index.snippet(0, &document), Some("Офис, переговорная 2"));
}

#[test]
fn exact_titles_precede_prefixes_and_description_matches() {
    let mut document = sample();
    let category = document.events[0].category;
    for (id, title, notes) in [
        (2, "План", ""),
        (3, "План поездки", ""),
        (4, "Бюджет", "План"),
    ] {
        let mut item = event(id, category, document.events[0].schedule);
        item.title = Title::try_from(title).unwrap();
        item.notes = notes.into();
        document.events.push(item);
    }
    let index = search(&document, "план", SearchFilters::default());
    let ids: Vec<_> = index.results().iter().map(|hit| hit.event.0).collect();
    assert_eq!(ids[0], 2);
    assert!(
        ids.iter().position(|id| *id == 3).unwrap() < ids.iter().position(|id| *id == 4).unwrap()
    );
}

#[test]
fn title_prefix_keeps_priority_when_other_tokens_match_a_longer_location() {
    let mut document = sample();
    let mut item = event(2, document.events[0].category, document.events[0].schedule);
    item.title = Title::try_from("Обсуждение плана").unwrap();
    item.location = "Офис".into();
    document.events.push(item);
    assert_eq!(
        search(&document, "план офис", SearchFilters::default()).results()[0].event,
        EventId(1)
    );
}

#[test]
fn highlight_indices_follow_graphemes_and_literal_punctuation_is_searchable() {
    let mut document = sample();
    document.events[0].title = Title::try_from("👩‍💻Cafe\u{301} — ВСТРЕЧА").unwrap();
    let query = SearchQuery::parse("cafe").unwrap();
    let mut index = search(&document, "cafe", SearchFilters::default());
    let mut indices = Vec::new();
    index.highlight(&query, 0, HighlightField::Title, &mut indices);
    assert_eq!(indices, [1, 2, 3, 4]);
    document.events[0].title = Title::try_from("ab:notes aaab-bbbbb ^план").unwrap();
    for input in ["ab:notes", "aaab-bbbbb", "^план"] {
        assert_eq!(
            search(&document, input, SearchFilters::default())
                .results()
                .len(),
            1
        );
    }
}

#[test]
fn snippet_highlighting_resolves_its_field_and_clears_a_reused_buffer() {
    let document = sample();
    let query = SearchQuery::parse("план офис").unwrap();
    let mut index = search(&document, "план офис", SearchFilters::default());
    let mut indices = Vec::new();
    index.highlight(&query, 0, HighlightField::Snippet, &mut indices);
    assert_eq!(indices, [0, 1, 2, 3]);
    let query = SearchQuery::parse("план").unwrap();
    index = search(&document, "план", SearchFilters::default());
    assert!(index.snippet(0, &document).is_none());
    index.highlight(&query, 0, HighlightField::Snippet, &mut indices);
    assert_eq!(indices, [] as [u32; 0]);
}

#[test]
fn calendar_and_group_names_are_searchable_and_visibility_is_explicit() {
    let mut document = sample();
    let name = document
        .category(document.events[0].category)
        .unwrap()
        .name
        .get(document.language)
        .to_owned();
    assert_eq!(
        search(&document, &name, SearchFilters::default())
            .results()
            .len(),
        1
    );
    let group = document
        .groups
        .iter_mut()
        .find(|group| {
            group
                .categories
                .iter()
                .any(|category| category.id == document.events[0].category)
        })
        .unwrap();
    group.name = crate::text::Name::custom("Исследования").unwrap();
    group.enabled = false;
    assert_eq!(
        search(&document, "исследования", SearchFilters::default())
            .results()
            .len(),
        1
    );
    assert!(
        search(
            &document,
            "план",
            SearchFilters {
                calendars: CalendarScope::Visible,
                ..SearchFilters::default()
            }
        )
        .results()
        .is_empty()
    );
    assert_eq!(
        search(
            &document,
            "план",
            SearchFilters {
                calendars: CalendarScope::Category(document.events[0].category),
                ..SearchFilters::default()
            }
        )
        .results()
        .len(),
        1
    );
    assert!(
        search(
            &document,
            "план",
            SearchFilters {
                status: StatusScope::Value(Some(EventStatus::Cancelled)),
                ..SearchFilters::default()
            }
        )
        .results()
        .is_empty()
    );
}

#[test]
fn date_queries_use_overlap_and_intersect_the_explicit_range() {
    let document = sample();
    assert_eq!(
        search(&document, "14.10.2026", SearchFilters::default())
            .results()
            .len(),
        1
    );
    assert!(
        search(&document, "2026-10-15", SearchFilters::default())
            .results()
            .is_empty()
    );
    assert!(
        search(
            &document,
            "2026-10-14",
            SearchFilters {
                dates: Some(range("2026-11-01", "2026-11-30")),
                ..SearchFilters::default()
            }
        )
        .results()
        .is_empty()
    );
    for (input, error) in [
        ("2026-02-30", QueryError::Date),
        ("2101-01-01", QueryError::Date),
        ("25:00", QueryError::Time),
        ("2026-01-01 2026-01-02", QueryError::Dates),
        ("10:00 11:00", QueryError::Times),
    ] {
        assert!(matches!(SearchQuery::parse(input), Err(actual) if actual == error));
    }
}

#[test]
fn time_filters_follow_the_display_timezone_and_exclude_all_day_events() {
    let mut document = sample();
    document.display_timezone = DisplayTimeZone::Named(chrono_tz::Europe::Moscow);
    document.events[0].schedule = EventSchedule::timed(
        range("2026-10-12", "2026-10-12"),
        EventTimes {
            start: "22:00".parse().unwrap(),
            end: "23:00".parse().unwrap(),
        },
    )
    .unwrap()
    .with_timezone(EventTimeZone::Utc)
    .unwrap();
    assert_eq!(
        search(&document, "2026-10-13 01:00", SearchFilters::default())
            .results()
            .len(),
        1
    );
    assert!(
        search(&document, "2026-10-12", SearchFilters::default())
            .results()
            .is_empty()
    );
    assert!(
        search(
            &document,
            "план",
            SearchFilters {
                after: Some("02:00".parse().unwrap()),
                ..SearchFilters::default()
            }
        )
        .results()
        .is_empty()
    );
    let category = document.events[0].category;
    document.events.push(event(
        2,
        category,
        EventSchedule::all_day(range("2026-10-13", "2026-10-13")),
    ));
    assert_eq!(
        search(&document, "01:00", SearchFilters::default())
            .results()
            .len(),
        1
    );
}

#[test]
fn recurrence_is_grouped_globally_and_expanded_in_a_range_with_exclusions() {
    let mut document = sample();
    document.events[0].schedule = EventSchedule::all_day(range("2026-01-01", "2026-01-01"))
        .with_recurrence(Some(
            Recurrence::new(Frequency::Daily, NonZeroU16::new(1).unwrap(), None).unwrap(),
        ))
        .unwrap();
    document.events[0].identity.exclusions = vec![OccurrenceStart::Date(date("2026-10-03"))].into();
    let index = search(&document, "план", SearchFilters::default());
    assert_eq!(index.results().len(), 1);
    assert_eq!(index.results()[0].schedule.start_date(), date("2026-10-04"));
    let index = search(
        &document,
        "план",
        SearchFilters {
            dates: Some(range("2026-10-01", "2026-10-05")),
            ..SearchFilters::default()
        },
    );
    assert_eq!(index.results().len(), 4);
    assert!(
        index
            .results()
            .iter()
            .all(|hit| hit.schedule.start_date() != date("2026-10-03"))
    );
    document.events[0].schedule = document.events[0]
        .schedule
        .with_recurrence(Some(
            Recurrence::new(
                Frequency::Daily,
                NonZeroU16::new(1).unwrap(),
                Some(date("2026-01-03")),
            )
            .unwrap(),
        ))
        .unwrap();
    assert_eq!(
        search(&document, "план", SearchFilters::default()).results()[0]
            .schedule
            .start_date(),
        date("2026-01-03")
    );
}

#[test]
fn grouped_timed_series_uses_the_next_occurrence_after_a_finished_meeting() {
    let mut document = sample();
    document.events[0].schedule = EventSchedule::timed(
        range("2026-10-01", "2026-10-01"),
        EventTimes {
            start: "10:00".parse().unwrap(),
            end: "11:00".parse().unwrap(),
        },
    )
    .unwrap()
    .with_recurrence(Some(
        Recurrence::new(Frequency::Daily, NonZeroU16::MIN, None).unwrap(),
    ))
    .unwrap();
    assert_eq!(
        search(&document, "план", SearchFilters::default()).results()[0]
            .schedule
            .start_date(),
        date("2026-10-04")
    );
    document.events[0].schedule = document.events[0]
        .schedule
        .with_recurrence(Some(
            Recurrence::new(Frequency::Daily, NonZeroU16::MIN, Some(date("2026-10-03"))).unwrap(),
        ))
        .unwrap();
    assert_eq!(
        search(&document, "план", SearchFilters::default()).results()[0]
            .schedule
            .start_date(),
        date("2026-10-03")
    );
}

#[test]
fn nearest_occurrence_distinguishes_both_hours_when_clocks_go_backward() {
    let mut document = sample();
    document.display_timezone = DisplayTimeZone::Named(chrono_tz::Europe::Berlin);
    document.events[0].schedule = EventSchedule::timed(
        range("2026-10-24", "2026-10-24"),
        EventTimes {
            start: "01:10".parse().unwrap(),
            end: "01:20".parse().unwrap(),
        },
    )
    .unwrap()
    .with_timezone(EventTimeZone::Utc)
    .unwrap()
    .with_recurrence(Some(
        Recurrence::new(Frequency::Daily, NonZeroU16::MIN, None).unwrap(),
    ))
    .unwrap();
    let query = SearchQuery::parse("план").unwrap();
    let mut index = SearchIndex::default();
    index.restart();
    index.step(
        &document,
        &query,
        SearchFilters::default(),
        date("2026-10-25").and_hms_opt(0, 50, 0).unwrap().and_utc(),
    );
    assert_eq!(index.results()[0].schedule.start_date(), date("2026-10-25"));
}

#[test]
fn incremental_search_restarts_cleanly_after_edits_and_deletions() {
    let mut document = sample();
    let category = document.events[0].category;
    for id in 2..=600 {
        document
            .events
            .push(event(id, category, document.events[0].schedule));
    }
    let query = SearchQuery::parse("план").unwrap();
    let mut index = SearchIndex::default();
    index.restart();
    index.step(
        &document,
        &query,
        SearchFilters::default(),
        date("2026-10-03").and_hms_opt(12, 0, 0).unwrap().and_utc(),
    );
    assert!(index.is_pending());
    let other = SearchQuery::parse("Test").unwrap();
    index.restart();
    while index.is_pending() {
        index.step(
            &document,
            &other,
            SearchFilters::default(),
            date("2026-10-03").and_hms_opt(12, 0, 0).unwrap().and_utc(),
        );
    }
    assert_eq!(index.results().len(), 599);
    document.events.clear();
    index.invalidate();
    index.step(
        &document,
        &other,
        SearchFilters::default(),
        date("2026-10-03").and_hms_opt(12, 0, 0).unwrap().and_utc(),
    );
    assert!(!index.is_pending());
    assert!(index.results().is_empty());
}
