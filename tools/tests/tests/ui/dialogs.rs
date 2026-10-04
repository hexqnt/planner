use super::*;

#[test]
fn new_event_focuses_title_once_and_accepts_typing_without_a_click() {
    for language in [Language::Russian, Language::English] {
        for repeated_pass in [false, true] {
            let mut harness = harness(
                Document {
                    language,
                    ..document()
                },
                egui::vec2(1440.0, 960.0),
            );
            if repeated_pass {
                harness.ctx.add_plugin(RepeatedPass::default());
            }
            let title = language.text("Придумайте название", "Add a title");
            for _ in 0..2 {
                new_event(&mut harness, language);
                assert!(field(&harness, title).is_focused());
                harness.event(egui::Event::Text("Встреча Meeting".into()));
                harness.run();
                assert_eq!(
                    field(&harness, title).value().as_deref(),
                    Some("Встреча Meeting")
                );
                let start = language.text("Дата начала", "Start date");
                focus_field(&mut harness, start);
                harness.run_steps(3);
                assert!(field(&harness, start).is_focused());
                click(&mut harness, language.text("Отмена", "Cancel"));
                assert!(!state(&harness).event_editor_open);
            }
        }
    }
}

#[test]
fn event_calendar_choice_is_remembered_after_cancel_and_used_for_next_event() {
    for language in [Language::Russian, Language::English] {
        let mut harness = harness(
            Document {
                language,
                ..document()
            },
            egui::vec2(1440.0, 960.0),
        );
        harness.ctx.add_plugin(RepeatedPass::default());
        let first = state(&harness).document.groups[0].categories[0]
            .name
            .get(language)
            .to_owned();
        let other = &state(&harness).document.groups[0].categories[1];
        let id = other.id;
        let name = other.name.get(language).to_owned();
        new_event(&mut harness, language);
        harness
            .get_by(|node| {
                node.role() == Role::ComboBox
                    && node.value().is_some_and(|value| value.contains(&first))
            })
            .click();
        harness.run();
        harness.get_by_role_and_label(Role::Button, &name).click();
        harness.run();
        assert_eq!(state(&harness).document.last_event_category, Some(id));
        click(&mut harness, language.text("Отмена", "Cancel"));
        assert!(state(&harness).document.events.is_empty());
        new_event(&mut harness, language);
        harness.get_by(|node| {
            node.role() == Role::ComboBox && node.value().is_some_and(|value| value.contains(&name))
        });
        type_into(
            &mut harness,
            language.text("Придумайте название", "Add a title"),
            "Meeting",
        );
        click(&mut harness, language.text("Сохранить", "Save"));
        assert_eq!(state(&harness).document.events[0].category, id);
    }
}

#[test]
fn event_timezone_can_be_searched_saved_and_reopened_without_changing_display_zone() {
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
            "план Meeting",
        );
        click(&mut harness, language.text("Весь день", "All day"));
        harness
            .get_by(|node| node.role() == Role::ComboBox && node.value().as_deref() == Some("UTC"))
            .click();
        harness.run();
        type_into(
            &mut harness,
            language.text("Поиск города или зоны", "Search city or timezone"),
            "Tokyo",
        );
        click(&mut harness, "Asia/Tokyo");
        click(&mut harness, language.text("Сохранить", "Save"));
        assert_eq!(
            state(&harness).document.display_timezone,
            testing::DisplayTimeZone::Utc
        );
        assert_eq!(
            state(&harness).document.events[0]
                .schedule
                .timezone()
                .name(),
            "Asia/Tokyo"
        );
        open_search(&mut harness);
        type_into(
            &mut harness,
            language.text("Поиск событий…", "Search events…"),
            "Meeting",
        );
        click(&mut harness, language.text("Редактировать", "Edit"));
        harness.get_by(|node| {
            node.role() == Role::ComboBox && node.value().as_deref() == Some("Asia/Tokyo")
        });
        choose(
            &mut harness,
            "Asia/Tokyo",
            language.text("Местное время", "Local time"),
        );
        click(&mut harness, language.text("Сохранить", "Save"));
        assert_eq!(
            state(&harness).document.events[0]
                .schedule
                .timezone()
                .name(),
            "Floating"
        );
    }
}

#[test]
fn text_context_menu_preserves_selection_and_supports_editing_and_paste() {
    for language in [Language::Russian, Language::English] {
        let mut harness = harness(
            Document {
                language,
                ..document()
            },
            egui::vec2(1440.0, 960.0),
        );
        new_event(&mut harness, language);
        let title = language.text("Придумайте название", "Add a title");
        type_into(&mut harness, title, "Встреча 🌍");
        field(&harness, title).click_secondary();
        harness.run();
        click(&mut harness, language.text("Выделить всё", "Select all"));
        field(&harness, title).click_secondary();
        harness.run();
        harness
            .get_by_label(language.text("Копировать", "Copy"))
            .click();
        harness.step();
        harness.step();
        assert!(harness.output().platform_output.commands.iter().any(
            |command| matches!(command, egui::OutputCommand::CopyText(text) if text == "Встреча 🌍")
        ));
        harness.run();
        field(&harness, title).click_secondary();
        harness.run();
        click(&mut harness, language.text("Вырезать", "Cut"));
        assert_eq!(field(&harness, title).value().as_deref(), Some(""));
        field(&harness, title).click_secondary();
        harness.run();
        click(&mut harness, language.text("Отменить", "Undo"));
        assert_eq!(
            field(&harness, title).value().as_deref(),
            Some("Встреча 🌍")
        );
        field(&harness, title).click_secondary();
        harness.run();
        click(&mut harness, language.text("Повторить", "Redo"));
        assert_eq!(field(&harness, title).value().as_deref(), Some(""));
        field(&harness, title).click_secondary();
        harness.run();
        click(&mut harness, language.text("Отменить", "Undo"));
        field(&harness, title).click_secondary();
        harness.run();
        harness
            .get_by_label(language.text("Вставить", "Paste"))
            .click();
        harness.step();
        assert!(harness.output().viewport_output.values().any(|output| {
            output
                .commands
                .contains(&egui::ViewportCommand::RequestPaste)
        }));
        harness.event(egui::Event::Paste("Новое название".into()));
        harness.run();
        assert_eq!(
            field(&harness, title).value().as_deref(),
            Some("Новое название")
        );
        click(&mut harness, language.text("Сохранить", "Save"));
        assert_eq!(
            state(&harness).document.events[0].title.get(),
            "Новое название"
        );
    }
}

#[derive(Default)]
pub struct RepeatedPass {
    discarded: usize,
}

impl egui::Plugin for RepeatedPass {
    fn debug_name(&self) -> &'static str {
        "Repeated event editor pass"
    }

    fn on_begin_pass(&mut self, ui: &mut egui::Ui) {
        if ui.ctx().current_pass_index() == 0
            && ui.input(|input| {
                input
                    .events
                    .iter()
                    .any(|event| matches!(event, egui::Event::PointerButton { pressed: false, .. }))
            })
        {
            ui.ctx()
                .request_discard("Test repeated event editor rendering");
            self.discarded += 1;
        }
    }
}

fn discarded(harness: &AppHarness) -> usize {
    harness
        .ctx
        .with_plugin::<RepeatedPass, _>(|pass| pass.discarded)
        .unwrap()
}

#[test]
fn dialog_fits_small_screens_and_saves_from_the_footer() {
    for language in [Language::Russian, Language::English] {
        for dark in [false, true] {
            let size = egui::vec2(360.0, 600.0);
            let mut harness = harness(
                Document {
                    language,
                    dark,
                    ..document()
                },
                size,
            );
            new_event(&mut harness, language);
            type_into(
                &mut harness,
                language.text("Придумайте название", "Add a title"),
                "Meeting",
            );
            click(&mut harness, language.text("Весь день", "All day"));
            let window = testing::event_dialog_rect(&harness.ctx).unwrap();
            assert!(
                egui::Rect::from_min_size(egui::Pos2::ZERO, size).contains_rect(window),
                "dark={dark}: {window:?}"
            );
            click(&mut harness, language.text("Сохранить", "Save"));
            assert!(!state(&harness).event_editor_open);
            assert_eq!(state(&harness).document.events.len(), 1);
            assert!(
                state(&harness).document.events[0]
                    .schedule
                    .times()
                    .is_some()
            );
        }
    }
}

#[test]
fn dialog_preserves_failed_edits_and_requires_delete_confirmation() {
    let mut document = search_document();
    let other = event(
        2,
        document.events[0].category,
        EventSchedule::all_day(range("2026-06-01", "2026-06-01")),
    );
    let untouched = serde_json::to_value(&other).unwrap();
    document.events.push(other);
    let mut harness = harness(document, egui::vec2(1440.0, 960.0));
    harness.ctx.add_plugin(RepeatedPass::default());
    let before = document_json(&harness);
    let id = state(&harness).document.events[0].id;
    edit_first_result(&mut harness);
    replace_text(&mut harness, "Придумайте название", "");
    click(&mut harness, "Сохранить");
    assert_eq!(state(&harness).event_error, Some(InputError::EmptyTitle));
    harness.get_by_label("Введите название события");
    assert_eq!(document_json(&harness), before);
    click(&mut harness, "Отмена");
    assert!(!state(&harness).event_editor_open);
    assert_eq!(document_json(&harness), before);
    click(&mut harness, "Редактировать");
    let passes = discarded(&harness);
    click(&mut harness, "Удалить");
    assert!(discarded(&harness) > passes);
    harness.get_by_label("Удалить навсегда");
    assert_eq!(document_json(&harness), before);
    let passes = discarded(&harness);
    click(&mut harness, "Удалить навсегда");
    assert!(discarded(&harness) > passes);
    assert!(!state(&harness).event_editor_open);
    assert!(
        state(&harness)
            .document
            .events
            .iter()
            .all(|event| event.id != id)
    );
    assert_eq!(state(&harness).document.events.len(), 1);
    assert_eq!(
        serde_json::to_value(&state(&harness).document.events[0]).unwrap(),
        untouched
    );
}

#[test]
fn deletion_confirmation_can_be_canceled_or_saved_without_changing_event_identity() {
    for save in [false, true] {
        let mut harness = harness(search_document(), egui::vec2(1440.0, 960.0));
        harness.ctx.add_plugin(RepeatedPass::default());
        let before = document_json(&harness);
        let id = state(&harness).document.events[0].id;
        let uid = state(&harness).document.events[0].identity.uid.clone();
        edit_first_result(&mut harness);
        click(&mut harness, "Удалить");
        harness.get_by_label("Удалить навсегда");
        assert_eq!(document_json(&harness), before);
        replace_text(&mut harness, "Придумайте название", "Updated meeting");
        let passes = discarded(&harness);
        click(
            &mut harness,
            if save {
                "Сохранить"
            } else {
                "Отмена"
            },
        );
        assert!(discarded(&harness) > passes);
        assert!(!state(&harness).event_editor_open);
        let state = state(&harness);
        assert_eq!(state.document.events.len(), 1);
        assert_eq!(state.document.events[0].id, id);
        assert_eq!(state.document.events[0].identity.uid, uid);
        if save {
            assert_eq!(state.document.events[0].title.get(), "Updated meeting");
        } else {
            assert_eq!(document_json(&harness), before);
        }
    }
}

#[test]
fn invalid_new_event_can_be_corrected_and_saved_once() {
    let mut harness = harness(document(), egui::vec2(1440.0, 960.0));
    new_event(&mut harness, Language::Russian);
    click(&mut harness, "Сохранить");
    assert_eq!(state(&harness).event_error, Some(InputError::EmptyTitle));
    assert!(state(&harness).document.events.is_empty());
    type_into(&mut harness, "Придумайте название", "Встреча");
    click(&mut harness, "Сохранить");
    assert!(!state(&harness).event_editor_open);
    assert_eq!(state(&harness).document.events.len(), 1);
    assert_eq!(state(&harness).document.events[0].title.get(), "Встреча");
    assert_eq!(
        state(&harness).selection,
        Some(state(&harness).document.events[0].schedule.occupied_dates())
    );
    harness.run_steps(3);
    assert_eq!(state(&harness).document.events.len(), 1);
}

#[test]
fn resizing_an_event_editor_preserves_the_draft_and_cancel_preserves_the_document() {
    let mut harness = harness(document(), egui::vec2(1440.0, 960.0));
    let before = document_json(&harness);
    new_event(&mut harness, Language::Russian);
    type_into(&mut harness, "Придумайте название", "Черновик");
    for size in [
        egui::vec2(820.0, 700.0),
        egui::vec2(1920.0, 1080.0),
        egui::vec2(1440.0, 960.0),
    ] {
        harness.set_size(size);
        harness.run();
        assert_eq!(
            field(&harness, "Придумайте название").value().as_deref(),
            Some("Черновик")
        );
        assert_eq!(document_json(&harness), before);
    }
    click(&mut harness, "Отмена");
    assert!(!state(&harness).event_editor_open);
    assert!(!state(&harness).dirty);
    assert_eq!(document_json(&harness), before);
}

#[test]
fn backup_and_notice_headers_close_without_replacing_the_document() {
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
            let before = document_json(&harness);
            let backup = language.text("Резервная копия JSON", "JSON backup");
            open_json_backup(&mut harness, language);
            harness
                .get_by_role_and_label(Role::Window, backup)
                .get_by_label("×")
                .click();
            harness.run();
            assert!(
                harness
                    .query_by_role_and_label(Role::Window, backup)
                    .is_none()
            );
            open_json_backup(&mut harness, language);
            let editor = harness.get_by_role(Role::MultilineTextInput);
            editor.focus();
            harness.run();
            assert!(harness.get_by_role(Role::MultilineTextInput).is_focused());
            harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
            harness.key_press(Key::Backspace);
            harness.run();
            harness.get_by_role(Role::MultilineTextInput).type_text("{");
            harness.run();
            assert_eq!(
                harness
                    .get_by_role(Role::MultilineTextInput)
                    .value()
                    .as_deref(),
                Some("{")
            );
            click(
                &mut harness,
                language.text("Заменить календарь", "Replace calendar"),
            );
            assert!(state(&harness).notice.is_some());
            harness
                .get_by_role_and_label(Role::Window, language.text("Сообщение", "Message"))
                .get_by_label("×")
                .click();
            harness.run();
            assert!(state(&harness).notice.is_none());
            harness
                .get_by_role_and_label(Role::Window, backup)
                .get_by_label("×")
                .click();
            harness.run();
            assert!(
                harness
                    .query_by_role_and_label(Role::Window, backup)
                    .is_none()
            );
            assert_eq!(document_json(&harness), before);
            assert!(!state(&harness).dirty);
        }
    }
}

#[test]
fn calendar_deletion_can_be_canceled_from_the_header_or_confirmed() {
    for language in [Language::Russian, Language::English] {
        for dark in [false, true] {
            let mut document = Document {
                language,
                dark,
                ..document()
            };
            let category = document.categories().next().unwrap();
            let id = category.id;
            let name = category.name.get(language).to_owned();
            document.events.push(event(
                1,
                id,
                EventSchedule::all_day(range("2026-01-12", "2026-01-16")),
            ));
            let mut harness = harness(document, egui::vec2(1440.0, 1600.0));
            let before = document_json(&harness);
            let title = language.text("Группы и календари", "Groups and calendars");
            for cancel in [Some("×"), Some(language.text("Отмена", "Cancel")), None] {
                open_tree_menu(&mut harness, &name);
                click(&mut harness, language.text("Удалить", "Delete"));
                if let Some(label) = cancel {
                    click(&mut harness, label);
                } else {
                    key(&mut harness, Key::Escape);
                }
                assert!(
                    harness
                        .query_by_role_and_label(Role::Window, title)
                        .is_none()
                );
                assert_eq!(document_json(&harness), before);
                assert!(!state(&harness).dirty);
            }
            open_tree_menu(&mut harness, &name);
            click(&mut harness, language.text("Удалить", "Delete"));
            click(
                &mut harness,
                language.text("Удалить навсегда", "Delete permanently"),
            );
            assert!(
                harness
                    .query_by_role_and_label(Role::Window, title)
                    .is_none()
            );
            assert!(state(&harness).document.category(id).is_none());
            assert!(state(&harness).document.events.is_empty());
            assert!(state(&harness).dirty);
        }
    }
}

#[test]
fn date_and_time_inputs_normalize_step_and_save_in_both_languages() {
    for language in [Language::Russian, Language::English] {
        let mut harness = harness(
            Document {
                language,
                ..document()
            },
            egui::vec2(1440.0, 960.0),
        );
        new_event(&mut harness, language);
        replace_text(
            &mut harness,
            language.text("Придумайте название", "Add a title"),
            "Meeting",
        );
        let start = language.text("Дата начала", "Start date");
        let end = language.text("Дата окончания", "End date");
        replace_text(&mut harness, start, "2026-10-02");
        focus_field(&mut harness, end);
        assert_eq!(
            field(&harness, start).value().as_deref(),
            Some(language.text("02.10.2026", "2026-10-02"))
        );
        replace_text(&mut harness, end, "02.10.2026");
        click(&mut harness, language.text("Весь день", "All day"));
        let start_time = language.text("Время начала", "Start time");
        let end_time = language.text("Время окончания", "End time");
        replace_text(&mut harness, start_time, "930");
        focus_field(&mut harness, end_time);
        assert_eq!(
            field(&harness, start_time).value().as_deref(),
            Some("09:30")
        );
        replace_text(&mut harness, end_time, "10:00:01.125");
        key(&mut harness, Key::Home);
        key(&mut harness, Key::ArrowUp);
        assert_eq!(
            field(&harness, end_time).value().as_deref(),
            Some("11:00:01.125")
        );
        key(&mut harness, Key::ArrowDown);
        click(&mut harness, language.text("Сохранить", "Save"));
        let saved = &state(&harness).document.events[0];
        assert_eq!(saved.schedule.dates(), range("2026-10-02", "2026-10-02"));
        let times = saved.schedule.times().unwrap();
        assert_eq!(times.start.to_string(), "09:30:00");
        assert_eq!(times.end.to_string(), "10:00:01.125");
    }
}

#[test]
fn invalid_date_is_reported_on_blur_and_can_be_corrected() {
    let mut harness = harness(document(), egui::vec2(1440.0, 960.0));
    new_event(&mut harness, Language::Russian);
    replace_text(&mut harness, "Дата начала", "31.02.2026");
    assert!(harness.query_by_label("Некорректная дата").is_none());
    focus_field(&mut harness, "Дата окончания");
    harness.get_by_label("Некорректная дата");
    replace_text(&mut harness, "Дата начала", "28.02.2026");
    focus_field(&mut harness, "Дата окончания");
    assert!(harness.query_by_label("Некорректная дата").is_none());
}

#[test]
fn arrow_steps_keep_the_date_order_and_short_time_cursor() {
    let mut harness = harness(document(), egui::vec2(1440.0, 960.0));
    new_event(&mut harness, Language::Russian);
    replace_text(&mut harness, "Дата начала", "2026-01-31");
    key(&mut harness, Key::Home);
    key(&mut harness, Key::ArrowUp);
    key(&mut harness, Key::ArrowUp);
    assert_eq!(
        field(&harness, "Дата начала").value().as_deref(),
        Some("2028-01-31")
    );
    focus_field(&mut harness, "Дата окончания");
    assert_eq!(
        field(&harness, "Дата начала").value().as_deref(),
        Some("31.01.2028")
    );
    click(&mut harness, "Весь день");
    replace_text(&mut harness, "Время начала", "930");
    key(&mut harness, Key::Home);
    key(&mut harness, Key::ArrowRight);
    key(&mut harness, Key::ArrowRight);
    key(&mut harness, Key::ArrowUp);
    key(&mut harness, Key::ArrowUp);
    assert_eq!(
        field(&harness, "Время начала").value().as_deref(),
        Some("09:32")
    );
}

fn wheel_over_field(harness: &mut AppHarness, label: &str, delta: egui::Vec2) {
    let pointer = field(harness, label).rect().center();
    harness.input_mut().events.extend([
        egui::Event::PointerMoved(pointer),
        egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            phase: egui::TouchPhase::Move,
            delta,
            modifiers: Modifiers::NONE,
        },
    ]);
    harness.run_steps(40);
}

#[test]
fn mouse_wheel_steps_the_focused_date_and_time_without_scrolling_the_form() {
    for language in [Language::Russian, Language::English] {
        let mut harness = harness(
            Document {
                language,
                ..document()
            },
            egui::vec2(960.0, 600.0),
        );
        new_event(&mut harness, language);
        let date = language.text("Дата начала", "Start date");
        let other = language.text("Дата окончания", "End date");
        replace_text(&mut harness, date, "2026-01-31");
        key(&mut harness, Key::Home);
        for _ in 0..5 {
            key(&mut harness, Key::ArrowRight);
        }
        let before = field(&harness, date).rect();
        wheel_over_field(&mut harness, date, egui::vec2(0.0, 1.0));
        assert_eq!(field(&harness, date).value().as_deref(), Some("2026-02-28"));
        assert_eq!(field(&harness, date).rect(), before);
        wheel_over_field(&mut harness, date, egui::vec2(0.0, -1.0));
        assert_eq!(field(&harness, date).value().as_deref(), Some("2026-01-28"));
        focus_field(&mut harness, other);
        let value = field(&harness, date).value();
        wheel_over_field(&mut harness, date, egui::vec2(0.0, 1.0));
        assert_eq!(field(&harness, date).value(), value);
        click(&mut harness, language.text("Весь день", "All day"));
        let time = language.text("Время начала", "Start time");
        replace_text(&mut harness, time, "09:30:01.125");
        key(&mut harness, Key::Home);
        for _ in 0..3 {
            key(&mut harness, Key::ArrowRight);
        }
        wheel_over_field(&mut harness, time, egui::vec2(0.0, 1.0));
        assert_eq!(
            field(&harness, time).value().as_deref(),
            Some("09:31:01.125")
        );
        wheel_over_field(&mut harness, time, egui::vec2(0.0, -1.0));
        assert_eq!(
            field(&harness, time).value().as_deref(),
            Some("09:30:01.125")
        );
        wheel_over_field(&mut harness, time, egui::vec2(1.0, 0.0));
        assert_eq!(
            field(&harness, time).value().as_deref(),
            Some("09:30:01.125")
        );
        let pointer = field(&harness, other).rect().center();
        harness
            .input_mut()
            .events
            .push(egui::Event::PointerMoved(pointer));
        harness.run();
        assert_eq!(field(&harness, date).rect(), before);
    }
}

#[test]
fn recurrence_end_modes_save_exclusive_limits_and_reopen() {
    for language in [Language::Russian, Language::English] {
        let mut harness = harness(
            Document {
                language,
                ..document()
            },
            egui::vec2(1440.0, 1200.0),
        );
        new_event(&mut harness, language);
        enlarge_event_editor(&mut harness);
        replace_text(
            &mut harness,
            language.text("Придумайте название", "Add a title"),
            "Repeated meeting",
        );
        replace_text(
            &mut harness,
            language.text("Дата начала", "Start date"),
            "2026-10-04",
        );
        replace_text(
            &mut harness,
            language.text("Дата окончания", "End date"),
            "2026-10-04",
        );
        choose(
            &mut harness,
            language.text("Не повторять", "Does not repeat"),
            language.text("Ежедневно", "Daily"),
        );
        choose(
            &mut harness,
            language.text("Без окончания", "No end"),
            language.text("До даты", "Until date"),
        );
        replace_text(
            &mut harness,
            language.text("Конец повторений", "Repeat until"),
            "10.10.2026",
        );
        choose(
            &mut harness,
            language.text("До даты", "Until date"),
            language.text("После N повторений", "After N occurrences"),
        );
        assert!(
            harness
                .query_by_role_and_label(
                    Role::TextInput,
                    language.text("Конец повторений", "Repeat until")
                )
                .is_none()
        );
        set_drag_value(
            &mut harness,
            language.text("Количество повторений", "Occurrence count"),
            "3",
        );
        click(&mut harness, language.text("Сохранить", "Save"));
        let rule = state(&harness).document.events[0]
            .schedule
            .recurrence()
            .unwrap();
        assert_eq!(rule.count().unwrap().get(), 3);
        assert!(rule.until().is_none());
        open_search(&mut harness);
        type_into(
            &mut harness,
            language.text("Поиск событий…", "Search events…"),
            "Repeated meeting",
        );
        click(&mut harness, language.text("Редактировать", "Edit"));
        choose(
            &mut harness,
            language.text("После N повторений", "After N occurrences"),
            language.text("До даты", "Until date"),
        );
        replace_text(
            &mut harness,
            language.text("Конец повторений", "Repeat until"),
            "2026-10-10",
        );
        click(&mut harness, language.text("Сохранить", "Save"));
        let rule = state(&harness).document.events[0]
            .schedule
            .recurrence()
            .unwrap();
        assert!(rule.count().is_none());
        assert_eq!(rule.until().unwrap().to_string(), "2026-10-10");
    }
}

fn replace_labelled_text(harness: &mut AppHarness, label: &str, text: &str) {
    harness.get_by_label(label).click();
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
    harness.key_press(Key::Backspace);
    harness.run();
    harness.get_by_label(label).type_text(text);
    harness.run();
}

#[test]
fn link_and_email_inputs_normalize_show_errors_and_preserve_failed_saves() {
    for language in [Language::Russian, Language::English] {
        let mut harness = harness(
            Document {
                language,
                ..document()
            },
            egui::vec2(1440.0, 1200.0),
        );
        new_event(&mut harness, language);
        enlarge_event_editor(&mut harness);
        let before = document_json(&harness);
        let title = language.text("Придумайте название", "Add a title");
        let link = language.text("Ссылка", "Link");
        let emails = language.text("Участники", "Participants");
        replace_text(&mut harness, title, "Typed fields");
        replace_labelled_text(&mut harness, emails, "ok@example.com; broken-email");
        replace_labelled_text(&mut harness, link, "example.com");
        focus_field(&mut harness, title);
        harness.get_by_label(InputError::Participant.message(language));
        harness.get_by_label(InputError::EventLink.message(language));
        assert_eq!(document_json(&harness), before);
        click(&mut harness, language.text("Сохранить", "Save"));
        assert!(state(&harness).event_editor_open);
        assert_eq!(state(&harness).event_error, Some(InputError::EventLink));
        assert_eq!(
            harness.get_by_label(emails).value().as_deref(),
            Some("ok@example.com; broken-email")
        );
        assert_eq!(
            harness.get_by_label(link).value().as_deref(),
            Some("example.com")
        );
        assert_eq!(document_json(&harness), before);
        replace_labelled_text(&mut harness, link, "  HTTPS://EXAMPLE.COM/call  ");
        focus_field(&mut harness, title);
        assert_eq!(
            harness.get_by_label(link).value().as_deref(),
            Some("https://example.com/call")
        );
        click(&mut harness, language.text("Сохранить", "Save"));
        assert_eq!(state(&harness).event_error, Some(InputError::Participant));
        assert_eq!(
            harness.get_by_label(link).value().as_deref(),
            Some("https://example.com/call")
        );
        replace_labelled_text(
            &mut harness,
            emails,
            " a+b@example.com;\nfirst.last@EXAMPLE.com, ",
        );
        focus_field(&mut harness, title);
        assert_eq!(
            harness.get_by_label(emails).value().as_deref(),
            Some("a+b@example.com\nfirst.last@EXAMPLE.com")
        );
        click(&mut harness, language.text("Сохранить", "Save"));
        assert!(!state(&harness).event_editor_open);
        let saved = document_json(&harness);
        let restored = Document::parse(&saved.to_string()).unwrap();
        let event = &restored.events[0];
        assert_eq!(
            event.link.as_ref().unwrap().as_str(),
            "https://example.com/call"
        );
        assert_eq!(event.details.participants.len(), 2);
        for (email, expected) in event
            .details
            .participants
            .iter()
            .zip(["a+b@example.com", "first.last@EXAMPLE.com"])
        {
            assert_eq!(email.as_str(), expected);
        }
        open_search(&mut harness);
        type_into(
            &mut harness,
            language.text("Поиск событий…", "Search events…"),
            "Typed fields",
        );
        click(&mut harness, language.text("Редактировать", "Edit"));
        enlarge_event_editor(&mut harness);
        assert_eq!(
            harness.get_by_label(link).value().as_deref(),
            Some("https://example.com/call")
        );
        assert_eq!(
            harness.get_by_label(emails).value().as_deref(),
            Some("a+b@example.com\nfirst.last@EXAMPLE.com")
        );
        replace_labelled_text(&mut harness, link, "");
        replace_labelled_text(&mut harness, emails, "");
        click(&mut harness, language.text("Сохранить", "Save"));
        let event = &state(&harness).document.events[0];
        assert!(event.link.is_none());
        assert_eq!(event.details.participants.as_slice(), []);
    }
}

fn set_drag_value(harness: &mut AppHarness, label: &str, text: &str) {
    harness.get_by_label(label).click();
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
    harness.key_press(Key::Backspace);
    harness.run();
    harness
        .input_mut()
        .events
        .push(egui::Event::Text(text.into()));
    harness.key_press(Key::Enter);
    harness.run();
}

#[test]
fn weekday_selector_keeps_one_day_and_reminder_units_preserve_minutes() {
    for language in [Language::Russian, Language::English] {
        let mut harness = harness(
            Document {
                language,
                ..document()
            },
            egui::vec2(1440.0, 1400.0),
        );
        new_event(&mut harness, language);
        enlarge_event_editor(&mut harness);
        replace_text(
            &mut harness,
            language.text("Придумайте название", "Add a title"),
            "Weekly meeting",
        );
        replace_text(
            &mut harness,
            language.text("Дата начала", "Start date"),
            "2026-10-05",
        );
        replace_text(
            &mut harness,
            language.text("Дата окончания", "End date"),
            "2026-10-05",
        );
        choose(
            &mut harness,
            language.text("Не повторять", "Does not repeat"),
            language.text("Еженедельно", "Weekly"),
        );
        click(&mut harness, language.text("Дни недели", "Weekdays"));
        for (ru, en) in [
            ("Вт", "Tu"),
            ("Ср", "We"),
            ("Чт", "Th"),
            ("Пт", "Fr"),
            ("Сб", "Sa"),
            ("Вс", "Su"),
        ] {
            click(&mut harness, language.text(ru, en));
        }
        click(&mut harness, language.text("Пн", "Mo"));
        click(&mut harness, language.text("+ Напоминание", "+ Reminder"));
        choose(
            &mut harness,
            language.text("мин.", "min."),
            language.text("ч.", "hours"),
        );
        set_drag_value(
            &mut harness,
            language.text("Интервал напоминания", "Reminder interval"),
            "1,5",
        );
        choose(
            &mut harness,
            language.text("ч.", "hours"),
            language.text("дн.", "days"),
        );
        click(&mut harness, language.text("Сохранить", "Save"));
        let event = &state(&harness).document.events[0];
        assert_eq!(
            u8::from(event.schedule.recurrence().unwrap().weekdays().unwrap()),
            1
        );
        assert_eq!(event.details.reminders[0].minutes(), 90);
        let restored = Document::parse(&document_json(&harness).to_string()).unwrap();
        assert_eq!(restored.events[0].details.reminders[0].minutes(), 90);
    }
}

fn enlarge_event_editor(harness: &mut AppHarness) {
    let window = testing::event_dialog_rect(&harness.ctx).unwrap();
    let start = window.right_bottom() - egui::vec2(6.0, 6.0);
    let end = start + egui::vec2(260.0, 500.0);
    for event in [
        egui::Event::PointerMoved(start),
        egui::Event::PointerButton {
            pos: start,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        },
        egui::Event::PointerMoved(end),
        egui::Event::PointerButton {
            pos: end,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        },
    ] {
        harness.input_mut().events.push(event);
        harness.run_steps(3);
    }
}
