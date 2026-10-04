use super::*;

pub fn check_hint(harness: &mut AppHarness, target: egui::Rect, text: &str) {
    let before = document_json(harness);
    assert!(harness.query_by_label(text).is_none());
    harness.event(egui::Event::PointerMoved(target.center()));
    harness.run_steps(40);
    let hint = harness
        .query_by_label(text)
        .unwrap_or_else(|| panic!("Expected a hover hint: {text}; target: {target:?}"));
    assert!(
        harness.ctx.content_rect().contains_rect(hint.rect()),
        "Hint outside the screen: {text}; {:?}",
        hint.rect()
    );
    assert!(!egui::Popup::is_any_open(&harness.ctx));
    harness.event(egui::Event::PointerMoved(egui::pos2(1.0, 1.0)));
    harness.run_steps(3);
    assert!(harness.query_by_label(text).is_none());
    assert_eq!(document_json(harness), before);
}

#[test]
fn calendar_explanations_use_existing_tabs_and_dates_in_both_themes() {
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
            assert!(
                harness
                    .query_by_label(language.text("О календарях", "About calendars"))
                    .is_none()
            );
            let target = harness
                .get_by_role_and_label(Role::Button, language.text("Календари", "Calendars"))
                .rect();
            check_hint(&mut harness, target, language.text("Календари — категории событий. Галочки показывают или скрывают их события на датах.", "Calendars are event categories. Checkboxes show or hide their events on dates."));
            let target = harness.get_by_label("2026-01-12").rect();
            check_hint(
                &mut harness,
                target,
                language.text(
                    "Выберите день · Shift + клик или перетаскивание — интервал",
                    "Choose a day · Shift + click or drag for a range",
                ),
            );
        }
    }
}

#[test]
fn recurrence_hint_preserves_the_event_draft() {
    for language in [Language::Russian, Language::English] {
        let mut harness = harness(
            Document {
                language,
                ..document()
            },
            egui::vec2(1440.0, 960.0),
        );
        new_event(&mut harness, language);
        type_into(
            &mut harness,
            language.text("Придумайте название", "Add a title"),
            "Meeting",
        );
        let target = harness
            .get_by(|node| {
                node.role() == Role::ComboBox
                    && node.value().as_deref()
                        == Some(language.text("Не повторять", "Does not repeat"))
            })
            .rect();
        check_hint(
            &mut harness,
            target,
            language.text(
                "Окончание повторов: без ограничения, до даты или по количеству. Изменяется вся серия.",
                "End repeats without a limit, on a date or after a count. Edits apply to the whole series.",
            ),
        );
        assert!(state(&harness).event_editor_open);
        click(&mut harness, language.text("Сохранить", "Save"));
        assert_eq!(state(&harness).document.events[0].title.get(), "Meeting");
    }
}

#[test]
fn backup_help_is_a_hover_only_image_and_keeps_the_restore_warning_visible() {
    for language in [Language::Russian, Language::English] {
        let mut harness = harness(
            Document {
                language,
                ..document()
            },
            egui::vec2(1440.0, 960.0),
        );
        open_json_backup(&mut harness, language);
        harness.get_by_label(language.text(
            "При восстановлении текущие данные будут заменены.",
            "Restoring replaces current data.",
        ));
        let target = harness
            .get_by_role_and_label(
                Role::Image,
                language.text("О резервной копии", "About JSON backup"),
            )
            .rect();
        check_hint(&mut harness, target, language.text("Скопируйте JSON в файл для резервной копии. Для восстановления вставьте его сюда и нажмите «Заменить календарь». Текущие данные будут заменены.", "Copy the JSON to a file for backup. To restore, paste it here and click ‘Replace calendar’. Current data will be replaced."));
        for pressed in [true, false] {
            harness.event(egui::Event::PointerButton {
                pos: target.center(),
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Modifiers::NONE,
            });
            harness.run_steps(3);
        }
        assert!(!egui::Popup::is_any_open(&harness.ctx));
        assert!(
            harness
                .query_by_role_and_label(
                    Role::Button,
                    language.text("О резервной копии", "About JSON backup")
                )
                .is_none()
        );
        harness.get_by_role(Role::MultilineTextInput);
        key(&mut harness, Key::Escape);
        assert!(harness.query_by_role(Role::MultilineTextInput).is_none());
    }
}

#[test]
fn search_help_fits_the_sidebar_and_the_small_screen_overlay() {
    for language in [Language::Russian, Language::English] {
        for size in [egui::vec2(1440.0, 960.0), egui::vec2(360.0, 600.0)] {
            let mut harness = harness(
                Document {
                    language,
                    ..document()
                },
                size,
            );
            open_search(&mut harness);
            let target = field(&harness, language.text("Поиск событий…", "Search events…")).rect();
            check_hint(&mut harness, target, language.text("Название, описание, место, email или ссылка. Можно добавить дату или время начала: 2026-10-12 10:00.", "Title, description, location, email or link. Add a date or start time: 2026-10-12 10:00."));
            assert!(state(&harness).search.open);
            click(&mut harness, language.text("Фильтры", "Filters"));
            let target = field(&harness, language.text("Начало с", "Starts at or after")).rect();
            check_hint(&mut harness, target, language.text("В часовом поясе просмотра; события на весь день исключаются при фильтре времени.", "In the display timezone; time filters exclude all-day events."));
            assert!(state(&harness).search.open);
        }
    }
}

#[test]
fn import_hint_uses_the_destination_picker_and_preserves_the_document() {
    for language in [Language::Russian, Language::English] {
        let mut harness = harness(
            Document {
                language,
                ..document()
            },
            egui::vec2(1440.0, 960.0),
        );
        click(&mut harness, language.text("Настройки", "Settings"));
        click(
            &mut harness,
            language.text("Импорт/экспорт ⏵", "Import/export ⏵"),
        );
        click(&mut harness, language.text("Импорт .ics", "Import .ics"));
        let destination = state(&harness)
            .document
            .visible_categories()
            .next()
            .unwrap()
            .name
            .get(language)
            .to_owned();
        let target = harness
            .get_by(|node| {
                node.role() == Role::ComboBox
                    && node
                        .value()
                        .is_some_and(|value| value.contains(&destination))
            })
            .rect();
        check_hint(&mut harness, target, language.text("Совпадающие UID обновляются. Новые события добавляются в выбранный календарь. Участники, вложения и напоминания пока не переносятся.", "Matching UIDs are updated. New events go into the selected calendar. Participants, attachments and reminders are not transferred yet."));
        harness.get_by_label(language.text("Целевой календарь", "Destination calendar"));
        click(&mut harness, "🚫 Cancel");
        assert!(!state(&harness).dirty);
    }
}
