use super::*;

fn event_document(language: Language, position: EventListPosition) -> Document {
    let mut document = Document {
        language,
        event_list_position: position,
        ..document()
    };
    let category = document.groups[0].categories[0].id;
    let schedule = EventSchedule::all_day(range("2026-01-12", "2026-01-12"));
    document.events = (1..=40)
        .map(|id| {
            let mut event = event(id, category, schedule);
            event.title = Title::try_from(format!("Event {id:02}")).unwrap();
            event
        })
        .collect();
    document
}

fn set_event_list_position(
    harness: &mut AppHarness,
    language: Language,
    position: EventListPosition,
) {
    click(harness, language.text("Настройки", "Settings"));
    click(harness, language.text("Список событий ⏵", "Event list ⏵"));
    let label = match position {
        EventListPosition::Left => language.text("Слева", "Left"),
        EventListPosition::Right => language.text("Справа", "Right"),
    };
    click(harness, label);
    assert!(!egui::Popup::is_any_open(&harness.ctx));
}

fn events_rect(harness: &AppHarness, language: Language) -> egui::Rect {
    harness
        .get_by_role_and_label(Role::Pane, language.text("Список событий", "Event list"))
        .rect()
}

#[test]
fn event_list_settings_and_header_move_one_list_and_preserve_the_document() {
    for language in [Language::Russian, Language::English] {
        let mut harness = harness(
            event_document(language, EventListPosition::Left),
            egui::vec2(1440.0, 1600.0),
        );
        let before = document_json(&harness);
        let original = harness.get_by_label("Event 01").rect();
        set_event_list_position(&mut harness, language, EventListPosition::Right);
        let panel = events_rect(&harness, language);
        assert!(panel.left() > original.right());
        assert_eq!(harness.get_all_by_label("Event 01").count(), 1);
        let mut expected = before.clone();
        expected["event_list_position"] = serde_json::json!("right");
        assert_eq!(document_json(&harness), expected);
        assert!(state(&harness).dirty);
        click(
            &mut harness,
            language.text("Перенести события влево", "Move events left"),
        );
        assert_eq!(document_json(&harness), before);
        assert!(
            harness
                .query_by_role_and_label(Role::Pane, language.text("Список событий", "Event list"))
                .is_none()
        );
        assert_eq!(
            harness.get_by_label("Event 01").rect().left(),
            original.left()
        );
        harness
            .get_by_label(language.text("Перенести события вправо", "Move events right"))
            .click();
        harness.step();
        assert_eq!(harness.get_all_by_label("Event 01").count(), 1);
        harness.run();
        assert_eq!(document_json(&harness), expected);
        set_event_list_position(&mut harness, language, EventListPosition::Left);
        assert_eq!(document_json(&harness), before);
    }
}

#[test]
fn moving_events_left_restores_calendars_and_preserves_date_selection() {
    for language in [Language::Russian, Language::English] {
        let document = event_document(language, EventListPosition::Right);
        let category = document.groups[0].categories[0]
            .name
            .get(language)
            .to_owned();
        let mut harness = harness(document, egui::vec2(1440.0, 1600.0));
        click(&mut harness, "2026-01-12");
        let selection = state(&harness).selection;
        let mut expected = document_json(&harness);
        expected["event_list_position"] = serde_json::json!("left");
        open_search(&mut harness);
        click(&mut harness, "☰");
        click(
            &mut harness,
            language.text("Перенести события влево", "Move events left"),
        );
        assert!(!state(&harness).search.open);
        assert_eq!(state(&harness).selection, selection);
        assert_eq!(document_json(&harness), expected);
        harness.get_by_role_and_label(Role::CheckBox, &category);
        harness.get_by_label("Event 01");
        assert!(state(&harness).dirty);
    }
}

#[test]
fn event_list_remains_visible_when_calendars_are_hidden_or_search_is_open() {
    for language in [Language::Russian, Language::English] {
        for position in [EventListPosition::Left, EventListPosition::Right] {
            let document = event_document(language, position);
            let category = document.groups[0].categories[0]
                .name
                .get(language)
                .to_owned();
            let mut harness = harness(document, egui::vec2(1440.0, 1600.0));
            let before = document_json(&harness);
            assert!(
                harness
                    .query_by_label(language.text("Скрыть панель событий", "Hide events panel"))
                    .is_none()
            );
            assert!(
                harness
                    .get_by_role_and_label(
                        Role::Pane,
                        language.text("Действия календаря", "Calendar actions")
                    )
                    .query_by_label(language.text("События", "Events"))
                    .is_none()
            );
            for _ in 0..2 {
                click(&mut harness, "☰");
                assert!(
                    harness
                        .query_by_role_and_label(Role::CheckBox, &category)
                        .is_none()
                );
                assert_eq!(harness.get_all_by_label("Event 01").count(), 1);
                events_rect(&harness, language);
                click(&mut harness, "☰");
                harness.get_by_role_and_label(Role::CheckBox, &category);
                assert_eq!(harness.get_all_by_label("Event 01").count(), 1);
                assert_eq!(
                    harness
                        .query_by_role_and_label(
                            Role::Pane,
                            language.text("Список событий", "Event list")
                        )
                        .is_some(),
                    position == EventListPosition::Right
                );
                open_search(&mut harness);
                events_rect(&harness, language);
                assert_eq!(harness.get_all_by_label("Event 01").count(), 1);
                click(&mut harness, language.text("Календари", "Calendars"));
                assert!(!state(&harness).search.open);
                assert_eq!(harness.get_all_by_label("Event 01").count(), 1);
                open_search(&mut harness);
                key(&mut harness, Key::Escape);
                assert!(!state(&harness).search.open);
                assert_eq!(harness.get_all_by_label("Event 01").count(), 1);
            }
            assert_eq!(document_json(&harness), before);
            assert!(!state(&harness).dirty);
        }
    }
}

#[test]
fn temporary_right_panel_restores_left_placement_without_changing_the_document() {
    for language in [Language::Russian, Language::English] {
        let mut harness = harness(
            event_document(language, EventListPosition::Left),
            egui::vec2(1440.0, 1600.0),
        );
        let before = document_json(&harness);
        for search in [false, true] {
            if search {
                open_search(&mut harness);
            } else {
                click(&mut harness, "☰");
            }
            events_rect(&harness, language);
            click(
                &mut harness,
                language.text("Перенести события влево", "Move events left"),
            );
            assert!(
                harness
                    .query_by_role_and_label(
                        Role::Pane,
                        language.text("Список событий", "Event list")
                    )
                    .is_none()
            );
            assert!(!state(&harness).search.open);
            harness.get_by_label("Event 01");
            assert_eq!(document_json(&harness), before);
            assert!(!state(&harness).dirty);
        }
    }
}

#[test]
fn right_event_list_edits_creates_and_tracks_date_selection() {
    for language in [Language::Russian, Language::English] {
        let mut harness = harness(
            event_document(language, EventListPosition::Right),
            egui::vec2(1440.0, 960.0),
        );
        click(&mut harness, "Event 01");
        assert!(state(&harness).event_editor_open);
        click(&mut harness, language.text("Отмена", "Cancel"));
        click(&mut harness, "2026-01-13");
        assert_eq!(
            state(&harness).selection,
            Some(range("2026-01-13", "2026-01-13"))
        );
        assert!(harness.query_by_label("Event 01").is_none());
        let panel = harness
            .get_by_role_and_label(Role::Pane, language.text("Список событий", "Event list"));
        panel.get_by_label(
            language.text("В этом интервале нет событий.", "No events in this range."),
        );
        panel
            .get_by_label(language.text("+ Событие", "+ Event"))
            .click();
        harness.run();
        assert!(state(&harness).event_editor_open);
        assert_eq!(
            field(&harness, language.text("Дата начала", "Start date"))
                .value()
                .as_deref(),
            Some("2026-01-13")
        );
        click(&mut harness, language.text("Отмена", "Cancel"));
        click(
            &mut harness,
            language.text("Показать весь год", "Show whole year"),
        );
        assert!(state(&harness).selection.is_none());
        harness.get_by_label("Event 01");
    }
}

#[test]
fn events_scroll_keeps_header_and_calendar_tree_stationary() {
    let language = Language::Russian;
    let mut harness = harness(
        event_document(language, EventListPosition::Right),
        egui::vec2(1440.0, 960.0),
    );
    let header = harness.get_by_label("Перенести события влево").rect();
    let category = state(&harness).document.groups[0].categories[0]
        .name
        .get(language)
        .to_owned();
    let tree = harness
        .get_by_role_and_label(Role::CheckBox, &category)
        .rect();
    let pointer = harness.get_by_label("Event 01").rect().center();
    harness.input_mut().events.extend([
        egui::Event::PointerMoved(pointer),
        egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            phase: egui::TouchPhase::Move,
            delta: egui::vec2(0.0, -800.0),
            modifiers: Modifiers::NONE,
        },
    ]);
    harness.run_steps(30);
    harness.remove_cursor();
    harness.run();
    assert!(harness.query_by_label("Event 01").is_none());
    assert_eq!(
        harness.get_by_label("Перенести события влево").rect(),
        header
    );
    assert_eq!(
        harness
            .get_by_role_and_label(Role::CheckBox, &category)
            .rect(),
        tree
    );
    assert_eq!(state(&harness).document.year.get(), 2026);
    assert!(!state(&harness).dirty);
}

#[test]
fn right_panel_resizes_and_stays_docked_on_narrow_windows() {
    let language = Language::Russian;
    let mut document = event_document(language, EventListPosition::Right);
    document.view_mode = CalendarViewMode::Continuous;
    let mut harness = harness(document, egui::vec2(1440.0, 960.0));
    let original = events_rect(&harness, language);
    let border = egui::pos2(original.left() - 18.0, original.center().y);
    harness.drag_at(border);
    harness.run_steps(2);
    harness.hover_at(border - egui::vec2(100.0, 0.0));
    harness.run_steps(2);
    harness.drop_at(border - egui::vec2(100.0, 0.0));
    harness.run();
    let resized = events_rect(&harness, language);
    assert!(
        resized.width() > original.width() + 50.0,
        "{original:?} -> {resized:?}"
    );
    assert!(state(&harness).calendar.unwrap().rect.right() <= resized.left());
    click(&mut harness, "☰");
    harness.set_size(egui::vec2(760.0, 960.0));
    harness.run();
    let narrow = events_rect(&harness, language);
    assert!(state(&harness).calendar.unwrap().rect.right() <= narrow.left());
    harness.get_by_label("Event 01");
    harness.set_size(egui::vec2(360.0, 600.0));
    harness.run();
    let calendar = state(&harness).calendar.unwrap().rect;
    let narrow = events_rect(&harness, language);
    assert!(calendar.width() > 0.0);
    assert!(calendar.right() <= narrow.left());
    harness.get_by_label("Event 01");
    click(&mut harness, "☰");
    let calendar = state(&harness).calendar.unwrap().rect;
    let narrow = events_rect(&harness, language);
    assert!(calendar.width() > 0.0);
    assert!(narrow.width() > 0.0);
    assert!(calendar.right() <= narrow.left());
    harness.get_by_label("Event 01");
    assert_eq!(
        state(&harness).document.event_list_position,
        EventListPosition::Right
    );
    assert!(!state(&harness).dirty);
}
