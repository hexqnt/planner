use super::*;
use crate::test_support::document;

struct UnusedStorage;

impl eframe::Storage for UnusedStorage {
    fn get_string(&self, _: &str) -> Option<String> {
        panic!("Native documents must not read UI storage");
    }

    fn set_string(&mut self, _: &str, _: String) {
        panic!("Native documents must not write UI storage");
    }

    fn remove_string(&mut self, _: &str) {
        panic!("Native documents must not remove UI storage keys");
    }

    fn flush(&mut self) {}
}

#[test]
fn document_and_existing_recovery_survive_restart_without_using_ui_storage() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(
        directory.path().join("document.recovery.json"),
        "original broken JSON",
    )
    .unwrap();
    let mut loaded = load(Some(directory.path().into()));
    assert!(loaded.error.is_none());
    assert!(!loaded.persistence.is_dirty());
    assert!(!directory.path().join("document.json").exists());
    loaded.document = document();
    loaded.persistence.mark_changed();
    loaded
        .persistence
        .save(&loaded.document, &mut UnusedStorage)
        .unwrap();
    let json = std::fs::read_to_string(directory.path().join("document.json")).unwrap();
    assert!(json.starts_with("{\n"));
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&json).unwrap(),
        serde_json::to_value(&loaded.document).unwrap()
    );
    let mut restored = load(Some(directory.path().into()));
    assert!(restored.error.is_none());
    assert!(!restored.persistence.is_dirty());
    assert_eq!(
        restored.persistence.recovery_json(),
        Some("original broken JSON")
    );
    restored.document.add_examples();
    restored.persistence.mark_changed();
    restored
        .persistence
        .save(&restored.document, &mut UnusedStorage)
        .unwrap();
    let restored = load(Some(directory.path().into()));
    assert_eq!(restored.document.events.len(), 10);
    assert_eq!(
        restored.persistence.recovery_json(),
        Some("original broken JSON")
    );
}

#[test]
fn corrupt_document_is_preserved_before_replacement() {
    let directory = tempfile::tempdir().unwrap();
    let original = "{broken JSON";
    std::fs::write(directory.path().join("document.json"), original).unwrap();
    let mut loaded = load(Some(directory.path().into()));
    assert!(loaded.error.is_some());
    assert_eq!(loaded.persistence.recovery_json(), Some(original));
    loaded
        .persistence
        .save(&loaded.document, &mut UnusedStorage)
        .unwrap();
    assert!(!directory.path().join("document.recovery.json").exists());
    assert_eq!(
        std::fs::read_to_string(directory.path().join("document.json")).unwrap(),
        original
    );
    loaded.persistence.mark_changed();
    loaded
        .persistence
        .save(&loaded.document, &mut UnusedStorage)
        .unwrap();
    let restored = load(Some(directory.path().into()));
    assert!(restored.error.is_none());
    assert_eq!(restored.persistence.recovery_json(), Some(original));
}

#[test]
fn failed_save_retains_dirty_state_for_retry() {
    let directory = tempfile::tempdir().unwrap();
    let mut loaded = load(Some(directory.path().into()));
    loaded.document = document();
    loaded.persistence.mark_changed();
    std::fs::create_dir(directory.path().join("document.json")).unwrap();
    assert!(
        loaded
            .persistence
            .save(&loaded.document, &mut UnusedStorage)
            .is_err()
    );
    assert!(loaded.persistence.is_dirty());
    std::fs::remove_dir(directory.path().join("document.json")).unwrap();
    loaded
        .persistence
        .save(&loaded.document, &mut UnusedStorage)
        .unwrap();
    assert!(!loaded.persistence.is_dirty());
    let restored = load(Some(directory.path().into()));
    assert_eq!(
        serde_json::to_value(restored.document).unwrap(),
        serde_json::to_value(loaded.document).unwrap()
    );
}

#[test]
fn file_read_failure_prevents_overwriting_existing_data() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("document.json");
    std::fs::write(&path, [0xff]).unwrap();
    let mut loaded = load(Some(directory.path().into()));
    assert!(loaded.error.is_some());
    loaded.persistence.mark_changed();
    assert!(
        loaded
            .persistence
            .save(&loaded.document, &mut UnusedStorage)
            .is_err()
    );
    assert_eq!(std::fs::read(path).unwrap(), [0xff]);
}
