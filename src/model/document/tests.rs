use super::*;
use crate::{
    model::{EventSchedule, EventUid},
    test_support::{document, event, range},
};

fn populated_document() -> Document {
    let mut document = document();
    let schedule = EventSchedule::all_day(range("2026-10-03", "2026-10-05"));
    document.events = vec![
        event(1, CategoryId(1), schedule),
        event(2, CategoryId(7), schedule),
    ];
    document
}

#[test]
fn remembered_event_calendar_handles_legacy_stale_and_deleted_categories() {
    let mut document = document();
    let id = document.groups[0].categories[1].id;
    document.last_event_category = Some(id);
    let json = serde_json::to_value(&document).unwrap();
    assert_eq!(
        serde_json::from_value::<Document>(json.clone())
            .unwrap()
            .last_event_category,
        Some(id)
    );
    let mut legacy = json.clone();
    legacy
        .as_object_mut()
        .unwrap()
        .remove("last_event_category");
    assert_eq!(
        serde_json::from_value::<Document>(legacy)
            .unwrap()
            .last_event_category,
        None
    );
    let mut stale = json;
    stale["last_event_category"] = serde_json::json!(99999);
    assert_eq!(
        serde_json::from_value::<Document>(stale)
            .unwrap()
            .last_event_category,
        None
    );
    document.remove_category(id);
    assert_eq!(document.last_event_category, None);
    document.last_event_category = Some(document.groups[0].categories[0].id);
    document.remove_group(0);
    assert_eq!(document.last_event_category, None);
}

#[test]
fn document_fields_round_trip() {
    for view_mode in [CalendarViewMode::SingleYear, CalendarViewMode::Continuous] {
        let mut document = populated_document();
        document.view_mode = view_mode;
        document.event_list_position = EventListPosition::Right;
        document.region = Region::Naive;
        document.language = Language::English;
        document.dark = true;
        document.display_timezone = DisplayTimeZone::Named(chrono_tz::Europe::Moscow);
        document
            .recent_timezones
            .remember(chrono_tz::Europe::Berlin);
        document
            .recent_timezones
            .remember(chrono_tz::Europe::Moscow);
        let expected = serde_json::to_value(&document).unwrap();
        for restored in [
            Document::parse(&expected.to_string()).unwrap(),
            serde_json::from_value::<Document>(expected.clone()).unwrap(),
        ] {
            assert_eq!(serde_json::to_value(restored).unwrap(), expected);
        }
    }
}

#[test]
fn examples_have_unique_ids_and_valid_category_references() {
    for language in [Language::Russian, Language::English] {
        let mut document = document();
        document.language = language;
        document.add_examples();
        assert_eq!(document.events.len(), 10);
        assert!(
            document
                .events
                .iter()
                .all(|event| document.year.range().overlaps(event.schedule.dates()))
        );
        assert!(Document::parse(&serde_json::to_string(&document).unwrap()).is_ok());
    }
}

#[test]
fn legacy_documents_default_each_missing_display_setting() {
    let mut document = document();
    document.view_mode = CalendarViewMode::Continuous;
    document.event_list_position = EventListPosition::Right;
    document.display_timezone = DisplayTimeZone::Named(chrono_tz::Europe::Moscow);
    document
        .recent_timezones
        .remember(chrono_tz::Europe::Berlin);
    let value = serde_json::to_value(&document).unwrap();
    for (field, default) in [
        ("view_mode", serde_json::json!(CalendarViewMode::SingleYear)),
        (
            "event_list_position",
            serde_json::json!(EventListPosition::Left),
        ),
        (
            "display_timezone",
            serde_json::json!(DisplayTimeZone::System),
        ),
        ("recent_timezones", serde_json::json!([])),
    ] {
        let mut legacy = value.clone();
        legacy.as_object_mut().unwrap().remove(field);
        let restored = Document::parse(&legacy.to_string()).unwrap();
        let mut expected = value.clone();
        expected[field] = default;
        assert_eq!(serde_json::to_value(restored).unwrap(), expected, "{field}");
    }
}

#[test]
fn all_deserialization_paths_reject_invalid_documents_for_the_expected_reason() {
    let value = serde_json::to_value(populated_document()).unwrap();
    for (path, replacement, reason) in [
        (
            "/version",
            serde_json::json!(2),
            "Unsupported document version",
        ),
        ("/year", serde_json::json!(2101), "Year must be between"),
        (
            "/groups/0/name/ru",
            serde_json::json!(" \t "),
            "Name is required",
        ),
        (
            "/groups/0/categories/0/name/en",
            serde_json::json!(""),
            "Name is required",
        ),
        (
            "/groups/1/categories/0/id",
            serde_json::json!(1),
            "Duplicate category ID",
        ),
        ("/events/1/id", serde_json::json!(1), "Duplicate event ID"),
        (
            "/events/1/identity/uid",
            value["events"][0]["identity"]["uid"].clone(),
            "Duplicate event UID",
        ),
        (
            "/events/0/identity/uid",
            serde_json::json!(""),
            "Invalid event UID",
        ),
        (
            "/events/0/identity/exclusions",
            serde_json::json!([{"Local": "2026-10-03T09:00:00"}]),
            "Exclusions must match",
        ),
        (
            "/events/0/category",
            serde_json::json!(999),
            "unknown category",
        ),
        (
            "/events/0/title",
            serde_json::json!(" "),
            "Event title is required",
        ),
        (
            "/events/0/range/end",
            serde_json::json!("2026-10-02"),
            "End date must not precede",
        ),
        (
            "/events/0/range/end",
            serde_json::json!("2026-02-30"),
            "out of range",
        ),
        (
            "/view_mode",
            serde_json::json!("unknown"),
            "unknown variant",
        ),
        (
            "/event_list_position",
            serde_json::json!("unknown"),
            "unknown variant",
        ),
        (
            "/display_timezone",
            serde_json::json!({"Named": "Unknown/Zone"}),
            "failed to parse timezone",
        ),
    ] {
        let mut invalid = value.clone();
        *invalid.pointer_mut(path).unwrap() = replacement;
        for result in [
            serde_json::from_value::<Document>(invalid.clone()),
            Document::parse(&invalid.to_string()),
        ] {
            let error = result
                .err()
                .unwrap_or_else(|| panic!("Accepted invalid {path}"));
            assert!(
                error.to_string().contains(reason),
                "{path}: {error}; expected {reason}"
            );
        }
    }
}

#[test]
fn batch_merge_preserves_ids_and_calendars_and_allocates_gaps() {
    let mut document = populated_document();
    document.events[0].id = EventId(0);
    document.events[1].id = EventId(u64::MAX);
    let mut update = event(
        1,
        CategoryId(3),
        EventSchedule::all_day(range("2026-11-01", "2026-11-02")),
    );
    update.title = Title::try_from("Updated").unwrap();
    let mut expected = serde_json::to_value(&update).unwrap();
    expected["id"] = serde_json::json!(0);
    expected["category"] = serde_json::json!(1);
    let schedule = update.schedule;
    document.merge_events(vec![
        event(3, CategoryId(7), schedule),
        update,
        event(4, CategoryId(2), schedule),
    ]);
    assert_eq!(
        document
            .events
            .iter()
            .map(|event| event.id.0)
            .collect::<Vec<_>>(),
        [0, u64::MAX, 1, 2]
    );
    assert_eq!(serde_json::to_value(&document.events[0]).unwrap(), expected);
    assert!(Document::parse(&serde_json::to_string(&document).unwrap()).is_ok());
}

#[test]
fn repeated_uids_within_a_batch_update_the_first_insert_and_last_update_wins() {
    let mut document = document();
    let schedule = EventSchedule::all_day(document.year.range());
    let first = event(10, CategoryId(7), schedule);
    let uid = first.identity.uid.clone();
    let mut last = event(10, CategoryId(1), schedule);
    last.title = Title::try_from("Last update").unwrap();
    document.merge_events(vec![first, event(20, CategoryId(2), schedule), last]);
    assert_eq!(document.events.len(), 2);
    assert_eq!(document.events[0].id, EventId(1));
    assert_eq!(document.events[0].category, CategoryId(7));
    assert_eq!(document.events[0].identity.uid, uid);
    assert_eq!(document.events[0].title.get(), "Last update");
    assert_eq!(document.events[1].id, EventId(2));
    assert_eq!(
        document.events[1].identity.uid,
        EventUid::try_from("event-20@tests".to_owned()).unwrap()
    );
    assert!(Document::parse(&serde_json::to_string(&document).unwrap()).is_ok());
}

#[test]
fn empty_merge_leaves_the_document_unchanged() {
    let mut document = populated_document();
    let before = serde_json::to_value(&document).unwrap();
    document.merge_events(Vec::new());
    assert_eq!(serde_json::to_value(&document).unwrap(), before);
}

#[test]
fn id_allocation_handles_gaps_zero_and_maximum_ids() {
    for (ids, expected) in [
        (&[][..], 1),
        (&[0, u64::MAX][..], 1),
        (&[3, 1, u64::MAX][..], 2),
        (&[2, 1, 3][..], 4),
    ] {
        assert_eq!(first_unused_id(ids.iter().copied()), expected, "{ids:?}");
    }
}

#[test]
fn category_visibility_requires_both_parent_and_child_to_be_enabled() {
    let mut document = document();
    let id = CategoryId(1);
    for group_enabled in [false, true] {
        for category_enabled in [false, true] {
            document.groups[0].enabled = group_enabled;
            document.groups[0].categories[0].enabled = category_enabled;
            assert!(document.category(id).is_some());
            assert_eq!(
                document.visible_category(id).is_some(),
                group_enabled && category_enabled
            );
            assert_eq!(
                document
                    .visible_categories()
                    .any(|category| category.id == id),
                group_enabled && category_enabled
            );
        }
    }
}

#[test]
fn deleted_category_ids_can_be_reused() {
    let mut document = document();
    document.remove_category(CategoryId(1));
    assert_eq!(document.next_category_id(), CategoryId(1));
}

#[test]
fn deleting_categories_and_groups_cascades_only_to_their_events() {
    let mut document = populated_document();
    let schedule = document.events[0].schedule;
    document.events.extend([
        event(3, CategoryId(2), schedule),
        event(4, CategoryId(9), schedule),
    ]);
    document.remove_category(CategoryId(7));
    assert!(document.category(CategoryId(7)).is_none());
    assert_eq!(
        document
            .events
            .iter()
            .map(|event| event.id.0)
            .collect::<Vec<_>>(),
        [1, 3, 4]
    );
    document.remove_group(0);
    assert_eq!(
        document
            .events
            .iter()
            .map(|event| event.id.0)
            .collect::<Vec<_>>(),
        [4]
    );
    assert_eq!(document.groups[0].name.get(Language::English), "Family");
    assert!(Document::parse(&serde_json::to_string(&document).unwrap()).is_ok());
    while !document.groups.is_empty() {
        document.remove_group(0);
    }
    assert!(document.events.is_empty());
    assert!(Document::parse(&serde_json::to_string(&document).unwrap()).is_ok());
}

#[test]
fn deleting_unknown_groups_and_categories_is_a_noop() {
    let mut document = populated_document();
    let before = serde_json::to_value(&document).unwrap();
    for index in [document.groups.len(), usize::MAX] {
        document.remove_group(index);
    }
    document.remove_category(CategoryId(u64::MAX));
    assert_eq!(serde_json::to_value(&document).unwrap(), before);
}

#[test]
fn language_mode_is_preserved_and_legacy_documents_keep_manual_language() {
    for mode in [LanguageMode::Manual, LanguageMode::Auto] {
        let mut document = document();
        document.language_mode = mode;
        document.language = Language::English;
        let mut json = serde_json::to_value(&document).unwrap();
        let mut restored = Document::parse(&json.to_string()).unwrap();
        assert_eq!(restored.language_mode, mode);
        restored.resolve_language();
        assert!(
            restored.language
                == if mode == LanguageMode::Auto {
                    Language::system()
                } else {
                    Language::English
                }
        );
        json.as_object_mut().unwrap().remove("language_mode");
        let restored = Document::parse(&json.to_string()).unwrap();
        assert_eq!(restored.language_mode, LanguageMode::Manual);
        assert!(restored.language == Language::English);
    }
}
