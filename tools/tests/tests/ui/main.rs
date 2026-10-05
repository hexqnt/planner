use egui::{Key, Modifiers, accesskit::Role};
use egui_kittest::{Node, kittest::Queryable as _};
use planner::testing::{
    self, AUTHORS, CalendarViewMode, Document, EventListPosition, EventSchedule, HOLIDAYS_VERSION,
    InputError, Language, REPOSITORY, Title, VERSION,
};
use planner_test_support::{AppHarness, document, event, harness, range, state};

mod dialogs;
mod events;
mod grid;
mod help;
mod search;
mod settings;
mod shortcuts;
mod sidebar;
mod vacation;
mod year;

fn click(harness: &mut AppHarness, label: &str) {
    harness.get_by_label(label).click();
    harness.run();
}

fn open_json_backup(harness: &mut AppHarness, language: Language) {
    click(harness, language.text("Настройки", "Settings"));
    click(
        harness,
        language.text("Импорт/экспорт ⏵", "Import/export ⏵"),
    );
    click(harness, language.text("Копия JSON", "JSON backup"));
}

fn open_tree_menu(harness: &mut AppHarness, name: &str) {
    let checkbox = harness.get_by_role_and_label(Role::CheckBox, name);
    let row_center = checkbox.rect().center().y;
    checkbox.hover();
    harness.run_steps(2);
    harness
        .get_all_by_label("…")
        .min_by(|left, right| {
            (left.rect().center().y - row_center)
                .abs()
                .total_cmp(&(right.rect().center().y - row_center).abs())
        })
        .unwrap()
        .click();
    harness.run();
}

fn field<'a>(harness: &'a AppHarness, label: &'a str) -> Node<'a> {
    harness.get_by_role_and_label(Role::TextInput, label)
}

fn focus_field(harness: &mut AppHarness, label: &str) {
    field(harness, label).click();
    harness.run();
    assert!(field(harness, label).is_focused());
}

fn type_into(harness: &mut AppHarness, label: &str, text: &str) {
    focus_field(harness, label);
    field(harness, label).type_text(text);
    harness.run();
}

fn replace_text(harness: &mut AppHarness, label: &str, text: &str) {
    focus_field(harness, label);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
    harness.key_press(Key::Backspace);
    harness.run();
    field(harness, label).type_text(text);
    harness.run();
}

fn key(harness: &mut AppHarness, key: Key) {
    harness.key_press(key);
    harness.run();
}

fn open_search(harness: &mut AppHarness) {
    harness.key_press_modifiers(Modifiers::COMMAND, Key::K);
    harness.run();
    assert!(state(harness).search.open);
}

fn search_document() -> Document {
    let mut document = document();
    let category = document.categories().next().unwrap().id;
    let mut item = event(
        1,
        category,
        EventSchedule::all_day(range("2030-10-12", "2030-10-14")),
    );
    item.title = Title::try_from("Планирование").unwrap();
    item.location = "Офис".into();
    document.events.push(item);
    document
}

fn document_json(harness: &AppHarness) -> serde_json::Value {
    serde_json::to_value(state(harness).document).unwrap()
}

fn edit_first_result(harness: &mut AppHarness) {
    open_search(harness);
    type_into(harness, "Поиск событий…", "план");
    click(harness, "Редактировать");
    assert!(state(harness).event_editor_open);
}

fn new_event(harness: &mut AppHarness, language: Language) {
    harness
        .get_by_role_and_label(
            Role::Pane,
            language.text("Действия календаря", "Calendar actions"),
        )
        .get_by_role_and_label(Role::Button, language.text("+ Событие", "+ Event"))
        .click();
    harness.run();
}

fn choose(harness: &mut AppHarness, value: &str, choice: &str) {
    harness
        .get_by(|node| node.role() == Role::ComboBox && node.value().as_deref() == Some(value))
        .click();
    harness.run();
    click(harness, choice);
}
