use eframe::Storage as _;
use rustc_hash::FxHashMap;

use super::*;
use crate::{
    model::{CategoryId, EventSchedule},
    test_support::{document, event, range},
};

#[derive(Default)]
struct Storage {
    values: FxHashMap<String, String>,
    writes: Vec<String>,
}

impl eframe::Storage for Storage {
    fn get_string(&self, key: &str) -> Option<String> {
        self.values.get(key).cloned()
    }

    fn set_string(&mut self, key: &str, value: String) {
        self.writes.push(key.into());
        self.values.insert(key.into(), value);
    }

    fn remove_string(&mut self, key: &str) {
        self.values.remove(key);
    }

    fn flush(&mut self) {}
}

impl Storage {
    fn with_document(json: String) -> Self {
        Self {
            values: FxHashMap::from_iter([(DOCUMENT_KEY.into(), json)]),
            ..Self::default()
        }
    }
}

#[test]
fn remembered_calendar_survives_restart_and_falls_back_after_deletion() {
    use crate::app::Planner;
    let ctx = egui::Context::default();
    let mut planner = Planner::from_document(document(), &ctx, None);
    let id = planner.document.groups[0].categories[1].id;
    planner.remember_event_category(id);
    let mut storage = Storage::default();
    planner
        .persistence
        .save(&planner.document, &mut storage)
        .unwrap();
    let mut restored = Planner::from_storage(Some(&storage), &ctx);
    assert_eq!(restored.document.last_event_category, Some(id));
    restored.new_event();
    assert_eq!(restored.event_draft.as_ref().unwrap().category(), id);
    restored.document.remove_category(id);
    restored.new_event();
    assert_ne!(restored.event_draft.as_ref().unwrap().category(), id);
}

#[test]
fn legacy_event_uids_are_persisted_before_the_next_restart() {
    let mut document = document();
    let schedule = EventSchedule::all_day(range("2026-10-03", "2026-10-03"));
    document.events = (1..=3)
        .map(|id| event(id, CategoryId(1), schedule))
        .collect();
    let mut legacy = serde_json::to_value(&document).unwrap();
    legacy["events"][0]
        .as_object_mut()
        .unwrap()
        .remove("identity");
    legacy["events"][1]["identity"]
        .as_object_mut()
        .unwrap()
        .remove("uid");
    let mut storage = Storage::with_document(legacy.to_string());
    let LoadedDocument {
        document,
        mut persistence,
        error,
    } = Persistence::load(Some(&storage));
    assert!(error.is_none());
    assert!(persistence.is_dirty());
    assert_eq!(storage.writes, Vec::<String>::new());
    assert_eq!(document.events[2].identity.uid.as_str(), "event-3@tests");
    let expected = serde_json::to_value(&document).unwrap();
    assert_eq!(
        persistence.save(&document, &mut storage).unwrap(),
        SaveOutcome::Written
    );
    assert_eq!(storage.writes, [DOCUMENT_KEY]);
    let restored = Persistence::load(Some(&storage));
    assert_eq!(serde_json::to_value(&restored.document).unwrap(), expected);
    assert!(restored.error.is_none());
    assert!(!restored.persistence.is_dirty());
    assert!(Document::parse(&storage.get_string(DOCUMENT_KEY).unwrap()).is_ok());
}

#[test]
fn documents_with_existing_uids_and_empty_documents_load_without_migration() {
    let mut document = document();
    for populated in [false, true] {
        if populated {
            document.events.push(event(
                1,
                CategoryId(1),
                EventSchedule::all_day(document.year.range()),
            ));
        }
        let expected = serde_json::to_value(&document).unwrap();
        let mut storage = Storage::with_document(expected.to_string());
        let LoadedDocument {
            document,
            mut persistence,
            error,
        } = Persistence::load(Some(&storage));
        assert!(error.is_none());
        assert!(!persistence.is_dirty());
        assert_eq!(serde_json::to_value(&document).unwrap(), expected);
        assert_eq!(
            persistence.save(&document, &mut storage).unwrap(),
            SaveOutcome::Unchanged
        );
        assert_eq!(storage.writes, Vec::<String>::new());
    }
}

#[test]
fn chosen_view_mode_survives_restart_and_backup_restore() {
    use crate::{app::Planner, model::CalendarViewMode};
    let ctx = egui::Context::default();
    let mut storage = Storage::default();
    let mut planner = Planner::from_document(Document::default(), &ctx, None);
    for mode in [CalendarViewMode::Continuous, CalendarViewMode::SingleYear] {
        planner.change_view_mode(mode);
        planner
            .persistence
            .save(&planner.document, &mut storage)
            .unwrap();
        let restored = Planner::from_storage(Some(&storage), &ctx);
        assert_eq!(restored.document.view_mode, mode);
        let backup = serde_json::to_string(&planner.document).unwrap();
        planner.replace_document(Document::parse(&backup).unwrap(), &ctx);
        assert_eq!(planner.document.view_mode, mode);
    }
}

#[test]
fn display_timezone_and_recent_choices_survive_restart_and_backup_restore() {
    use crate::{app::Planner, model::DisplayTimeZone};
    let ctx = egui::Context::default();
    let mut storage = Storage::default();
    let mut planner = Planner::from_document(Document::default(), &ctx, None);
    for zone in [
        DisplayTimeZone::Named(chrono_tz::Europe::Moscow),
        DisplayTimeZone::Utc,
        DisplayTimeZone::System,
    ] {
        planner.change_display_timezone(zone);
        planner
            .persistence
            .save(&planner.document, &mut storage)
            .unwrap();
        let restored = Planner::from_storage(Some(&storage), &ctx);
        assert_eq!(restored.document.display_timezone, zone);
        assert_eq!(
            restored
                .document
                .recent_timezones
                .iter()
                .collect::<Vec<_>>(),
            [chrono_tz::Europe::Moscow]
        );
        let backup = serde_json::to_string(&planner.document).unwrap();
        planner.replace_document(Document::parse(&backup).unwrap(), &ctx);
        assert_eq!(planner.document.display_timezone, zone);
    }
}

#[test]
fn corrupt_document_survives_save_without_edits_and_is_preserved_before_replacement() {
    let mut invalid = serde_json::to_value(document()).unwrap();
    invalid["version"] = serde_json::json!(2);
    for original in ["{invalid JSON".to_owned(), invalid.to_string()] {
        let mut storage = Storage::default();
        storage.values.insert(DOCUMENT_KEY.into(), original.clone());
        let LoadedDocument {
            document,
            mut persistence,
            error,
        } = Persistence::load(Some(&storage));
        assert!(error.is_some());
        assert_eq!(persistence.recovery_json(), Some(original.as_str()));
        assert_eq!(
            persistence.save(&document, &mut storage).unwrap(),
            SaveOutcome::Unchanged
        );
        assert_eq!(storage.get_string(DOCUMENT_KEY).unwrap(), original);
        assert_eq!(storage.writes, Vec::<String>::new());
        persistence.mark_changed();
        assert_eq!(
            persistence.save(&document, &mut storage).unwrap(),
            SaveOutcome::Written
        );
        assert_eq!(storage.writes, [RECOVERY_KEY, DOCUMENT_KEY]);
        assert_eq!(storage.get_string(RECOVERY_KEY).unwrap(), original);
        let LoadedDocument {
            document: restored,
            mut persistence,
            error,
        } = Persistence::load(Some(&storage));
        assert!(error.is_none());
        assert_eq!(
            serde_json::to_value(&restored).unwrap(),
            serde_json::to_value(&document).unwrap()
        );
        assert_eq!(persistence.recovery_json(), Some(original.as_str()));
        persistence.mark_changed();
        persistence.save(&restored, &mut storage).unwrap();
        assert_eq!(storage.writes, [RECOVERY_KEY, DOCUMENT_KEY, DOCUMENT_KEY]);
        assert_eq!(storage.get_string(RECOVERY_KEY).unwrap(), original);
    }
}

#[test]
fn clean_document_is_not_serialized_or_written_again() {
    let mut storage = Storage::default();
    let LoadedDocument {
        mut document,
        mut persistence,
        error,
    } = Persistence::load(None);
    assert!(error.is_none());
    assert_eq!(
        persistence.save(&document, &mut storage).unwrap(),
        SaveOutcome::Unchanged
    );
    document.year = crate::model::Year::try_from(2026).unwrap();
    document.events.push(event(
        1,
        CategoryId(1),
        EventSchedule::all_day(document.year.range()),
    ));
    persistence.mark_changed();
    assert_eq!(
        persistence.save(&document, &mut storage).unwrap(),
        SaveOutcome::Written
    );
    assert_eq!(
        persistence.save(&document, &mut storage).unwrap(),
        SaveOutcome::Unchanged
    );
    assert_eq!(storage.writes, [DOCUMENT_KEY]);
    let LoadedDocument {
        document: restored,
        error,
        ..
    } = Persistence::load(Some(&storage));
    assert!(error.is_none());
    assert_eq!(
        serde_json::to_value(restored).unwrap(),
        serde_json::to_value(document).unwrap()
    );
}

#[test]
fn an_existing_recovery_copy_is_retained_when_saving_a_valid_or_missing_document() {
    for missing in [false, true] {
        let expected = document();
        let mut storage = Storage::with_document(serde_json::to_string(&expected).unwrap());
        storage
            .values
            .insert(RECOVERY_KEY.into(), "older recovery".into());
        if missing {
            storage.values.remove(DOCUMENT_KEY);
        }
        let LoadedDocument {
            document,
            mut persistence,
            error,
        } = Persistence::load(Some(&storage));
        assert!(error.is_none());
        assert_eq!(persistence.recovery_json(), Some("older recovery"));
        assert_eq!(
            persistence.save(&document, &mut storage).unwrap(),
            SaveOutcome::Unchanged
        );
        persistence.mark_changed();
        assert_eq!(
            persistence.save(&document, &mut storage).unwrap(),
            SaveOutcome::Written
        );
        assert_eq!(storage.writes, [DOCUMENT_KEY]);
        assert_eq!(
            storage.get_string(RECOVERY_KEY).as_deref(),
            Some("older recovery")
        );
    }
}

#[test]
fn a_new_corrupt_document_supersedes_an_older_recovery_only_after_an_edit() {
    let mut storage = Storage::with_document("{new corruption".into());
    storage
        .values
        .insert(RECOVERY_KEY.into(), "older recovery".into());
    let LoadedDocument {
        document,
        mut persistence,
        error,
    } = Persistence::load(Some(&storage));
    assert!(error.is_some());
    assert_eq!(persistence.recovery_json(), Some("{new corruption"));
    assert_eq!(
        persistence.save(&document, &mut storage).unwrap(),
        SaveOutcome::Unchanged
    );
    assert_eq!(
        storage.get_string(RECOVERY_KEY).as_deref(),
        Some("older recovery")
    );
    persistence.mark_changed();
    assert_eq!(
        persistence.save(&document, &mut storage).unwrap(),
        SaveOutcome::Written
    );
    assert_eq!(storage.writes, [RECOVERY_KEY, DOCUMENT_KEY]);
    assert_eq!(
        storage.get_string(RECOVERY_KEY).as_deref(),
        Some("{new corruption")
    );
}

#[test]
fn planner_save_and_import_keep_recovery_available_after_restart() {
    use eframe::App as _;

    let original = "{unreadable calendar";
    let mut storage = Storage::default();
    storage.values.insert(DOCUMENT_KEY.into(), original.into());
    let ctx = egui::Context::default();
    let mut planner = super::super::Planner::from_storage(Some(&storage), &ctx);
    planner.save(&mut storage);
    assert_eq!(storage.writes, Vec::<String>::new());
    let mut imported = Document::default();
    imported.add_examples();
    planner.replace_document(imported, &ctx);
    planner.save(&mut storage);
    let mut planner = super::super::Planner::from_storage(Some(&storage), &ctx);
    assert_eq!(planner.document.events.len(), 10);
    assert_eq!(planner.persistence.recovery_json(), Some(original));
    let output = ctx.run_ui(egui::RawInput::default(), |ui| planner.render(ui));
    assert!(output.shapes.iter().any(|shape| {
        matches!(&shape.shape, egui::Shape::Text(text) if text.galley.text() == "Исходный JSON")
    }));
    output.drop_without_applying_deltas();
    planner.save(&mut storage);
    assert_eq!(storage.writes, [RECOVERY_KEY, DOCUMENT_KEY]);
}
