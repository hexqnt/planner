use super::*;
use crate::model::{CategoryId, Document};
use crate::test_support::{document, event, range};

fn file(summary: &str) -> String {
    format!(
        "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:meeting@example.org\r\nSUMMARY:{summary}\r\nDTSTART;VALUE=DATE:20261005\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n"
    )
}

#[test]
fn importing_twice_updates_uid_and_keeps_local_id_and_calendar() {
    let ctx = egui::Context::default();
    let mut planner = Planner::from_document(document(), &ctx, None);
    planner.new_event();
    planner
        .import_calendar(&file("Original"), CategoryId(2))
        .unwrap();
    let id = planner.document.events[0].id;
    assert!(planner.persistence.is_dirty());
    assert!(planner.event_draft.is_none());
    planner
        .import_calendar(&file("Updated"), CategoryId(3))
        .unwrap();
    assert_eq!(planner.document.events.len(), 1);
    assert_eq!(planner.document.events[0].id, id);
    assert_eq!(planner.document.events[0].category, CategoryId(2));
    assert_eq!(planner.document.events[0].title.get(), "Updated");
    let exported = crate::ical::export(&planner.document).unwrap();
    planner.import_calendar(&exported, CategoryId(3)).unwrap();
    assert_eq!(planner.document.events.len(), 1);
    assert_eq!(planner.document.events[0].id, id);
    assert_eq!(planner.document.events[0].category, CategoryId(2));
    assert_eq!(planner.document.events[0].title.get(), "Updated");
}

#[test]
fn failed_import_preserves_document_editor_selection_and_clean_persistence() {
    let invalid_date = file("Bad").replace("20261005", "20260230");
    let invalid_batch = file("Updated").replace("END:VCALENDAR\r\n", "BEGIN:VEVENT\r\nUID:broken\r\nDTSTART;VALUE=DATE:20260230\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n");
    for (input, category, reason) in [
        (invalid_date, CategoryId(2), "VEVENT 1:"),
        (invalid_batch, CategoryId(2), "VEVENT 2:"),
        (
            file("Updated"),
            CategoryId(u64::MAX),
            "Destination calendar no longer exists",
        ),
    ] {
        let ctx = egui::Context::default();
        let mut document = document();
        document.merge_events(crate::ical::import(&file("Original"), CategoryId(1)).unwrap());
        let mut planner = Planner::from_document(document, &ctx, None);
        planner.new_event();
        let before = serde_json::to_value(&planner.document).unwrap();
        let selection = planner.selection;
        let error = planner.import_calendar(&input, category).unwrap_err();
        assert!(error.starts_with(reason), "{error}");
        assert_eq!(serde_json::to_value(&planner.document).unwrap(), before);
        assert_eq!(planner.selection, selection);
        assert!(planner.event_draft.is_some());
        assert!(!planner.persistence.is_dirty());
    }
}

#[test]
fn export_includes_hidden_calendars_and_keeps_existing_events() {
    let ctx = egui::Context::default();
    let mut document = document();
    let schedule = crate::model::EventSchedule::all_day(range("2026-10-03", "2026-10-04"));
    document.events = vec![
        event(1, CategoryId(1), schedule),
        event(2, CategoryId(7), schedule),
    ];
    let mut planner = Planner::from_document(document, &ctx, None);
    planner.document.groups[0].enabled = false;
    planner
        .import_calendar(&file("New"), CategoryId(1))
        .unwrap();
    assert_eq!(planner.document.events.len(), 3);
    let before = serde_json::to_value(&planner.document).unwrap();
    let exported = crate::ical::export(&planner.document).unwrap();
    let restored = crate::ical::import(&exported, CategoryId(1)).unwrap();
    assert_eq!(restored.len(), 3);
    for (source, restored) in planner.document.events.iter().zip(restored) {
        let mut expected = serde_json::to_value(source).unwrap();
        expected["id"] = serde_json::json!(0);
        expected["category"] = serde_json::json!(1);
        assert_eq!(serde_json::to_value(restored).unwrap(), expected);
    }
    assert_eq!(serde_json::to_value(&planner.document).unwrap(), before);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn cancelling_a_file_dialog_clears_the_pending_operation() {
    let ctx = egui::Context::default();
    let mut planner = Planner::from_document(Document::default(), &ctx, None);
    for import in [true, false] {
        if import {
            planner.import_file();
        } else {
            planner.export_file();
        }
        assert!(planner.files.pending.is_some());
        ctx.run_ui(
            egui::RawInput {
                events: vec![egui::Event::Key {
                    key: egui::Key::Escape,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
                ..Default::default()
            },
            |ui| planner.file_dialog(ui.ctx()),
        )
        .drop_without_applying_deltas();
        assert!(planner.files.pending.is_none());
        assert!(planner.document.events.is_empty());
        assert!(!planner.persistence.is_dirty());
    }
}
