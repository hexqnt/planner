use super::*;

#[test]
fn settings_menu_switches_both_calendar_modes() {
    for language in [Language::Russian, Language::English] {
        let mut harness = harness(
            Document {
                language,
                ..document()
            },
            egui::vec2(1440.0, 960.0),
        );
        for (label, expected) in [
            (
                language.text("Непрерывный", "Continuous"),
                CalendarViewMode::Continuous,
            ),
            (
                language.text("Один год", "Single year"),
                CalendarViewMode::SingleYear,
            ),
        ] {
            click(&mut harness, language.text("Настройки", "Settings"));
            click(&mut harness, label);
            assert_eq!(state(&harness).document.view_mode, expected);
            assert!(state(&harness).dirty);
            assert!(!egui::Popup::is_any_open(&harness.ctx));
        }
    }
}

#[test]
fn about_displays_metadata_and_opens_repository_without_editing_the_document() {
    for language in [Language::Russian, Language::English] {
        for dark in [false, true] {
            let mut harness = harness(
                Document {
                    language,
                    dark,
                    ..document()
                },
                egui::vec2(1440.0, 960.0),
            );
            let before = document_json(&harness);
            click(&mut harness, language.text("Настройки", "Settings"));
            click(&mut harness, language.text("О программе", "About"));
            assert!(state(&harness).about_open);
            for label in [
                "planner",
                language.text("Версия", "Version"),
                VERSION,
                language.text("Автор", "Author"),
                AUTHORS,
                "holidays-ru",
                HOLIDAYS_VERSION,
            ] {
                harness.get_by_label(label);
            }
            let window =
                harness.get_by_role_and_label(Role::Window, language.text("О программе", "About"));
            let center = window
                .get_by_role_and_label(Role::Label, "planner")
                .rect()
                .center()
                .y;
            for rect in [
                window
                    .get_by_role_and_label(Role::Image, language.text("О программе", "About"))
                    .rect(),
                window.get_by_label("×").rect(),
            ] {
                assert!(
                    (rect.center().y - center).abs() <= 0.5,
                    "{rect:?}, title center {center}"
                );
            }
            harness
                .get_by_label(language.text("Исходный код на GitHub", "Source code on GitHub"))
                .click();
            // Команда открытия ссылки живёт только в кадре отпускания кнопки.
            harness.step();
            assert!(harness.output().platform_output.commands.iter().any(|command| {
                matches!(command, egui::OutputCommand::OpenUrl(url) if url.url == REPOSITORY)
            }));
            harness.run();
            click(&mut harness, language.text("Закрыть", "Close"));
            assert!(!state(&harness).about_open);
            click(&mut harness, language.text("Настройки", "Settings"));
            click(&mut harness, language.text("О программе", "About"));
            click(&mut harness, "×");
            assert!(!state(&harness).about_open);
            assert!(!state(&harness).dirty);
            assert_eq!(document_json(&harness), before);
        }
    }
}

#[test]
fn language_submenu_switches_manual_and_automatic_language() {
    use planner::testing::LanguageMode;

    for size in [egui::vec2(1440.0, 960.0), egui::vec2(760.0, 960.0)] {
        let mut harness = harness(document(), size);
        for choice in [
            Some(Language::English),
            Some(Language::Russian),
            None,
            Some(Language::English),
        ] {
            let language = state(&harness).document.language;
            assert!(harness.query_by_label("Русский").is_none());
            assert!(harness.query_by_label("English").is_none());
            click(&mut harness, language.text("Настройки", "Settings"));
            click(&mut harness, language.text("Язык ⏵", "Language ⏵"));
            for label in [language.text("Авто", "Auto"), "Русский", "English"] {
                harness.get_by_label(label);
            }
            let label = choice.map_or_else(
                || language.text("Авто", "Auto"),
                |language| language.text("Русский", "English"),
            );
            click(&mut harness, label);
            let expected_mode = if choice.is_some() {
                LanguageMode::Manual
            } else {
                LanguageMode::Auto
            };
            assert_eq!(state(&harness).document.language_mode, expected_mode);
            assert!(state(&harness).document.language == choice.unwrap_or_else(Language::system));
            assert!(state(&harness).dirty);
            assert!(!egui::Popup::is_any_open(&harness.ctx));
        }
    }
}

#[test]
fn import_export_submenu_opens_json_backup_without_changing_the_document() {
    for language in [Language::Russian, Language::English] {
        for size in [egui::vec2(1440.0, 960.0), egui::vec2(760.0, 960.0)] {
            let mut harness = harness(
                Document {
                    language,
                    ..document()
                },
                size,
            );
            let before = document_json(&harness);
            let label = language.text("Копия JSON", "JSON backup");
            assert!(harness.query_by_label(label).is_none());
            click(&mut harness, language.text("Настройки", "Settings"));
            assert!(harness.query_by_label(label).is_none());
            click(
                &mut harness,
                language.text("Импорт/экспорт ⏵", "Import/export ⏵"),
            );
            click(&mut harness, label);
            harness.get_by_role_and_label(
                Role::Window,
                language.text("Резервная копия JSON", "JSON backup"),
            );
            let json = harness
                .get_by_role(Role::MultilineTextInput)
                .value()
                .unwrap();
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(&json).unwrap(),
                before
            );
            assert!(!egui::Popup::is_any_open(&harness.ctx));
            assert!(!state(&harness).dirty);
            assert_eq!(document_json(&harness), before);
        }
    }
}

#[test]
fn import_export_submenu_opens_file_dialogs_without_changing_the_document() {
    for language in [Language::Russian, Language::English] {
        for size in [egui::vec2(1440.0, 960.0), egui::vec2(760.0, 960.0)] {
            for (label, title) in [
                (
                    language.text("Импорт .ics", "Import .ics"),
                    language.text("Импорт iCalendar", "Import iCalendar"),
                ),
                (
                    language.text("Экспорт .ics", "Export .ics"),
                    language.text("Экспорт iCalendar", "Export iCalendar"),
                ),
            ] {
                let mut harness = harness(
                    Document {
                        language,
                        ..document()
                    },
                    size,
                );
                let before = document_json(&harness);
                assert!(harness.query_by_label(label).is_none());
                click(&mut harness, language.text("Настройки", "Settings"));
                click(
                    &mut harness,
                    language.text("Импорт/экспорт ⏵", "Import/export ⏵"),
                );
                harness.get_by_label(language.text("Импорт .ics", "Import .ics"));
                harness.get_by_label(language.text("Экспорт .ics", "Export .ics"));
                click(&mut harness, label);
                harness.get_by_label(title);
                assert!(!egui::Popup::is_any_open(&harness.ctx));
                assert!(!state(&harness).dirty);
                assert_eq!(document_json(&harness), before);
                click(&mut harness, "🚫 Cancel");
                assert!(harness.query_by_label(title).is_none());
                assert!(!state(&harness).dirty);
                assert_eq!(document_json(&harness), before);
            }
        }
    }
}
