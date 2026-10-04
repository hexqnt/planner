use super::*;
use egui_kittest::kittest::NodeT as _;

#[test]
fn calendar_color_presets_only_change_the_selected_calendar_and_survive_serialization() {
    for language in [Language::Russian, Language::English] {
        for dark in [false, true] {
            let mut harness = harness(
                Document {
                    language,
                    dark,
                    ..document()
                },
                egui::vec2(1440.0, 1200.0),
            );
            let mut expected = document_json(&harness);
            harness
                .get_all_by_role_and_label(
                    Role::Button,
                    language.text("Цвет календаря", "Calendar color"),
                )
                .next()
                .unwrap()
                .click();
            harness.run();
            assert!(egui::Popup::is_any_open(&harness.ctx));
            click(&mut harness, "#11C168");
            expected["groups"][0]["categories"][0]["color"] = serde_json::json!([17, 193, 104]);
            assert_eq!(document_json(&harness), expected);
            assert!(state(&harness).dirty);
            let restored = Document::parse(&expected.to_string()).unwrap();
            assert_eq!(restored.groups[0].categories[0].color, [17, 193, 104]);
            key(&mut harness, Key::Escape);
            assert!(!egui::Popup::is_any_open(&harness.ctx));
        }
    }
}

#[test]
fn calendar_menu_creates_an_event_in_that_calendar() {
    for language in [Language::Russian, Language::English] {
        for enabled in [false, true] {
            let mut document = Document {
                language,
                ..document()
            };
            let category = &mut document.groups[0].categories[1];
            category.enabled = enabled;
            let id = category.id;
            let name = category.name.get(language).to_owned();
            let mut harness = harness(document, egui::vec2(1440.0, 1600.0));
            let mut before = document_json(&harness);
            before["last_event_category"] = serde_json::json!(id);
            for save in [false, true] {
                open_tree_menu(&mut harness, &name);
                harness
                    .get_by_label(language.text("Переименовать", "Rename"))
                    .parent()
                    .unwrap()
                    .get_by_label(language.text("+ Событие", "+ Event"))
                    .click();
                harness.run();
                assert!(state(&harness).event_editor_open);
                assert!(!egui::Popup::is_any_open(&harness.ctx));
                assert_eq!(document_json(&harness), before);
                assert!(state(&harness).dirty);
                type_into(
                    &mut harness,
                    language.text("Придумайте название", "Add a title"),
                    "Meeting",
                );
                click(
                    &mut harness,
                    if save {
                        language.text("Сохранить", "Save")
                    } else {
                        language.text("Отмена", "Cancel")
                    },
                );
                assert!(!state(&harness).event_editor_open);
                if save {
                    let state = state(&harness);
                    assert_eq!(state.document.events.len(), 1);
                    assert_eq!(state.document.events[0].category, id);
                    assert_eq!(state.document.events[0].title.get(), "Meeting");
                    assert!(state.dirty);
                } else {
                    assert_eq!(document_json(&harness), before);
                    assert!(state(&harness).dirty);
                }
            }
        }
    }
}

fn rename_text(harness: &mut AppHarness, text: &str) {
    assert!(harness.get_by_role(Role::TextInput).is_focused());
    harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
    harness.run();
    harness.get_by_role(Role::TextInput).type_text(text);
    harness.run();
}

#[test]
fn rename_context_menu_keeps_the_editor_open_and_restores_cut_text() {
    let mut harness = harness(document(), egui::vec2(1440.0, 960.0));
    let original = state(&harness).document.groups[0].categories[0]
        .name
        .get(Language::Russian)
        .to_owned();
    open_tree_menu(&mut harness, &original);
    click(&mut harness, "Переименовать");
    harness.get_by_role(Role::TextInput).click_secondary();
    harness.run();
    click(&mut harness, "Выделить всё");
    harness.get_by_role(Role::TextInput).click_secondary();
    harness.run();
    click(&mut harness, "Вырезать");
    assert_eq!(
        harness.get_by_role(Role::TextInput).value().as_deref(),
        Some("")
    );
    harness.get_by_role(Role::TextInput).click_secondary();
    harness.run();
    click(&mut harness, "Отменить");
    assert_eq!(
        harness.get_by_role(Role::TextInput).value().as_deref(),
        Some(original.as_str())
    );
    assert_eq!(
        state(&harness).document.groups[0].categories[0]
            .name
            .get(Language::Russian),
        original
    );
    key(&mut harness, Key::Escape);
    assert!(harness.query_by_role(Role::TextInput).is_none());
}

#[test]
fn tree_checkboxes_change_visibility_without_changing_events() {
    for language in [Language::Russian, Language::English] {
        for dark in [false, true] {
            let mut document = Document {
                language,
                dark,
                ..document()
            };
            let group_name = document.groups[0].name.get(language).to_owned();
            let category = &document.groups[0].categories[0];
            let id = category.id;
            let category_name = category.name.get(language).to_owned();
            document.events.push(event(
                1,
                id,
                EventSchedule::all_day(range("2026-01-12", "2026-01-16")),
            ));
            let mut harness = harness(document, egui::vec2(1440.0, 1600.0));
            let before = document_json(&harness);
            for name in [&group_name, &category_name] {
                click(&mut harness, name);
                assert!(state(&harness).document.visible_category(id).is_none());
                assert_eq!(document_json(&harness)["events"], before["events"]);
                assert!(state(&harness).dirty);
                click(&mut harness, name);
                assert!(state(&harness).document.visible_category(id).is_some());
                assert_eq!(document_json(&harness), before);
            }
        }
    }
}

#[test]
fn tree_creates_group_and_calendar_with_focused_name_editors() {
    for language in [Language::Russian, Language::English] {
        for dark in [false, true] {
            let mut harness = harness(
                Document {
                    language,
                    dark,
                    ..document()
                },
                egui::vec2(1440.0, 1600.0),
            );
            let groups = state(&harness).document.groups.len();
            click(
                &mut harness,
                language.text("+ Добавить группу", "+ Add group"),
            );
            assert!(harness.get_by_role(Role::TextInput).is_focused());
            assert_eq!(
                harness.get_by_role(Role::TextInput).value().as_deref(),
                Some(language.text("Новая группа", "New group")),
            );
            let group_name = language.text("Команда", "Team");
            harness.get_by_role(Role::TextInput).type_text(group_name);
            harness.run();
            key(&mut harness, Key::Enter);
            assert_eq!(state(&harness).document.groups.len(), groups + 1);
            assert_eq!(
                state(&harness).document.groups[groups].name.get(language),
                group_name,
            );
            assert!(harness.query_by_role(Role::TextInput).is_none());

            open_tree_menu(&mut harness, group_name);
            click(&mut harness, language.text("+ Календарь", "+ Calendar"));
            assert!(harness.get_by_role(Role::TextInput).is_focused());
            let category_name = language.text("Релизы", "Releases");
            harness
                .get_by_role(Role::TextInput)
                .type_text(category_name);
            harness.run();
            key(&mut harness, Key::Enter);
            let document = state(&harness).document;
            let group = &document.groups[groups];
            assert_eq!(group.categories.len(), 1);
            assert_eq!(group.categories[0].name.get(language), category_name);
            assert!(document.visible_category(group.categories[0].id).is_some());
            assert!(harness.query_by_role(Role::TextInput).is_none());
            assert!(!egui::Popup::is_any_open(&harness.ctx));
            assert!(state(&harness).dirty);
        }
    }
}

#[test]
fn tree_rename_cancels_with_escape_and_commits_with_enter() {
    for language in [Language::Russian, Language::English] {
        for group in [false, true] {
            let document = Document {
                language,
                ..document()
            };
            let name = if group {
                &document.groups[0].name
            } else {
                &document.groups[0].categories[0].name
            }
            .get(language)
            .to_owned();
            let mut harness = harness(document, egui::vec2(1440.0, 1600.0));
            let before = document_json(&harness);
            let updated = language.text("Планы команды", "Team plans");
            for commit in [false, true] {
                open_tree_menu(&mut harness, &name);
                click(&mut harness, language.text("Переименовать", "Rename"));
                rename_text(&mut harness, updated);
                key(&mut harness, if commit { Key::Enter } else { Key::Escape });
                assert!(harness.query_by_role(Role::TextInput).is_none());
                assert_eq!(state(&harness).dirty, commit);
                if commit {
                    harness.get_by_role_and_label(Role::CheckBox, updated);
                    let document = state(&harness).document;
                    let renamed = if group {
                        &document.groups[0].name
                    } else {
                        &document.groups[0].categories[0].name
                    };
                    assert_eq!(renamed.get(language), updated);
                } else {
                    assert_eq!(document_json(&harness), before);
                }
            }
        }
    }
}
