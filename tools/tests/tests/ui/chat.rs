use super::*;
use egui_kittest::kittest::NodeT as _;

#[test]
fn chat_sends_local_messages_and_starts_a_new_conversation_without_changing_calendar() {
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
            let input = language.text("Спросить о календаре…", "Ask about your calendar…");
            let send = language.text("Отправить", "Send");
            let new_chat = language.text("Новый чат", "New chat");
            click(&mut harness, language.text("Чат", "Chat"));
            harness
                .get_by_label(language.text("AI пока не подключён.", "AI is not connected yet."));
            assert!(harness.get_by_label(send).accesskit_node().is_disabled());
            assert!(
                harness
                    .get_by_label(language.text("Остановить", "Stop"))
                    .accesskit_node()
                    .is_disabled()
            );
            assert!(
                harness
                    .get_by_label(new_chat)
                    .accesskit_node()
                    .is_disabled()
            );

            type_into(&mut harness, input, "Plan a meeting tomorrow");
            click(&mut harness, send);
            harness.get_by_label("Plan a meeting tomorrow");
            assert_eq!(field(&harness, input).value().as_deref(), Some(""));
            assert!(field(&harness, input).is_focused());
            assert!(harness.get_by_label(send).accesskit_node().is_disabled());

            field(&harness, input).type_text("What is on this week?");
            harness.run();
            key(&mut harness, Key::Enter);
            harness.get_by_label("Plan a meeting tomorrow");
            harness.get_by_label("What is on this week?");
            assert_eq!(field(&harness, input).value().as_deref(), Some(""));

            type_into(&mut harness, input, "Unsent draft");
            click(&mut harness, new_chat);
            assert!(harness.query_by_label("Plan a meeting tomorrow").is_none());
            assert!(harness.query_by_label("What is on this week?").is_none());
            assert_eq!(field(&harness, input).value().as_deref(), Some(""));
            assert!(
                harness
                    .get_by_label(new_chat)
                    .accesskit_node()
                    .is_disabled()
            );
            assert_eq!(document_json(&harness), before);
            assert!(state(&harness).selection.is_none());
            assert!(!state(&harness).dirty);
        }
    }
}

#[test]
fn chat_preserves_history_and_draft_when_switching_to_calendars_and_search() {
    let mut harness = harness(document(), egui::vec2(1440.0, 960.0));
    click(&mut harness, "Чат");
    type_into(&mut harness, "Спросить о календаре…", "Message in history");
    key(&mut harness, Key::Enter);
    type_into(&mut harness, "Спросить о календаре…", "Draft with n and t");
    click(&mut harness, "Календари");
    assert!(harness.query_by_label("Спросить о календаре…").is_none());
    click(&mut harness, "Чат");
    harness.get_by_label("Message in history");
    assert_eq!(
        field(&harness, "Спросить о календаре…").value().as_deref(),
        Some("Draft with n and t")
    );

    open_search(&mut harness);
    assert!(harness.query_by_label("Спросить о календаре…").is_none());
    type_into(&mut harness, "Поиск событий…", "search draft");
    click(&mut harness, "Чат");
    assert!(!state(&harness).search.open);
    harness.get_by_label("Message in history");
    assert_eq!(
        field(&harness, "Спросить о календаре…").value().as_deref(),
        Some("Draft with n and t")
    );
    click(&mut harness, "Поиск");
    assert!(state(&harness).search.open);
    assert_eq!(state(&harness).search.query, "search draft");
}

#[test]
fn chat_ignores_blank_messages_and_shift_enter_inserts_a_new_line() {
    let mut harness = harness(document(), egui::vec2(1440.0, 960.0));
    click(&mut harness, "Чат");
    type_into(&mut harness, "Спросить о календаре…", "   ");
    assert!(
        harness
            .get_by_label("Отправить")
            .accesskit_node()
            .is_disabled()
    );
    key(&mut harness, Key::Enter);
    assert!(harness.query_by_label("ВЫ").is_none());
    replace_text(&mut harness, "Спросить о календаре…", "First line");
    harness.key_press_modifiers(Modifiers::SHIFT, Key::Enter);
    harness.run();
    field(&harness, "Спросить о календаре…").type_text("Second line");
    harness.run();
    assert_eq!(
        field(&harness, "Спросить о календаре…").value().as_deref(),
        Some("First line\nSecond line")
    );
    assert!(harness.query_by_label("ВЫ").is_none());
    key(&mut harness, Key::Enter);
    harness.get_by_label("First line\nSecond line");
    assert!(!state(&harness).event_editor_open);
}

#[test]
fn chat_sends_once_per_key_press_when_egui_repeats_a_pass() {
    let mut harness = harness(document(), egui::vec2(1440.0, 960.0));
    harness.ctx.add_plugin(super::shortcuts::RepeatKeyPass);
    let enter = |pressed| egui::Event::Key {
        key: Key::Enter,
        physical_key: Some(Key::Enter),
        pressed,
        repeat: false,
        modifiers: Modifiers::NONE,
    };
    let input = "Спросить о календаре…";
    click(&mut harness, "Чат");
    type_into(&mut harness, input, "Sent once");
    harness.event(enter(true));
    harness.run();
    assert_eq!(harness.get_all_by_label("Sent once").count(), 1);
    assert_eq!(field(&harness, input).value().as_deref(), Some(""));

    field(&harness, input).type_text("Next draft");
    harness.run();
    harness.event(enter(true));
    harness.run();
    assert!(harness.query_by_label("Next draft").is_none());
    assert_eq!(
        field(&harness, input).value().as_deref(),
        Some("Next draft")
    );

    harness.event(enter(false));
    harness.run();
    key(&mut harness, Key::Enter);
    assert_eq!(harness.get_all_by_label("Next draft").count(), 1);
    assert_eq!(field(&harness, input).value().as_deref(), Some(""));
}

#[test]
fn chat_composer_remains_accessible_in_a_narrow_window_with_a_long_history() {
    for language in [Language::Russian, Language::English] {
        let mut harness = harness(
            Document {
                language,
                event_list_position: EventListPosition::Left,
                ..document()
            },
            egui::vec2(900.0, 600.0),
        );
        let input = language.text("Спросить о календаре…", "Ask about your calendar…");
        let send = language.text("Отправить", "Send");
        click(&mut harness, language.text("Чат", "Chat"));
        for index in 0..12 {
            type_into(&mut harness, input, &format!("Calendar question {index}"));
            key(&mut harness, Key::Enter);
        }
        harness.get_by_label("Calendar question 11");
        let input_rect = field(&harness, input).rect();
        let send_rect = harness.get_by_label(send).rect();
        assert!(input_rect.is_positive());
        assert!(input_rect.bottom() <= send_rect.top());
        assert!(send_rect.bottom() < 600.0);
        click(&mut harness, language.text("Календари", "Calendars"));
        assert!(harness.query_by_label(input).is_none());
    }
}
