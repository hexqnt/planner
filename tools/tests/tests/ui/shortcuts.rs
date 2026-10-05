use super::*;
use chrono::Datelike as _;

#[derive(Clone, Copy)]
enum TextOrder {
    BeforeKey,
    AfterKey,
}

const fn key_event(key: Key, pressed: bool, modifiers: Modifiers) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: Some(key),
        pressed,
        repeat: false,
        modifiers,
    }
}

fn repeat_key(harness: &mut AppHarness, key: Key) {
    // egui определяет автоповтор по удерживаемой клавише, а не по флагу тестового события.
    harness.event(key_event(key, true, Modifiers::SHIFT));
    harness.event(key_event(key, true, Modifiers::NONE));
    harness.event(key_event(key, false, Modifiers::NONE));
    harness.run();
}

pub struct RepeatKeyPass;

impl egui::Plugin for RepeatKeyPass {
    fn debug_name(&self) -> &'static str {
        "Repeated keyboard pass"
    }

    fn on_begin_pass(&mut self, ui: &mut egui::Ui) {
        if ui.ctx().current_pass_index() == 0
            && ui.input(|input| {
                input
                    .events
                    .iter()
                    .any(|event| matches!(event, egui::Event::Key { pressed: true, .. }))
            })
        {
            ui.ctx().request_discard("Test repeated shortcut rendering");
        }
    }
}

fn description(harness: &AppHarness, language: Language) -> Node<'_> {
    harness.get_by(move |node| {
        node.role() == Role::MultilineTextInput
            && node.data().placeholder()
                == Some(language.text("Добавьте описание", "Add a description"))
    })
}

fn select_range(harness: &mut AppHarness) {
    click(harness, "2026-01-12");
    harness
        .get_by_label("2026-01-14")
        .click_modifiers(Modifiers::SHIFT);
    harness.run();
    assert_eq!(
        state(harness).selection,
        Some(range("2026-01-12", "2026-01-14"))
    );
}

fn press_letter(harness: &mut AppHarness, key: Key, text: &str) {
    press_letter_order(harness, key, text, TextOrder::AfterKey);
}

fn press_letter_order(harness: &mut AppHarness, key: Key, text: &str, order: TextOrder) {
    let mut events = [
        key_event(key, true, Modifiers::NONE),
        egui::Event::Text(text.into()),
    ];
    if matches!(order, TextOrder::BeforeKey) {
        events.swap(0, 1);
    }
    harness.input_mut().events.extend(events);
    harness
        .input_mut()
        .events
        .push(key_event(key, false, Modifiers::NONE));
    harness.run();
}

#[test]
fn new_event_shortcut_uses_selection_and_focuses_an_empty_title_in_both_layouts() {
    for language in [Language::Russian, Language::English] {
        for (text, order) in [
            ("n", TextOrder::AfterKey),
            ("т", TextOrder::AfterKey),
            ("n", TextOrder::BeforeKey),
            ("т", TextOrder::BeforeKey),
        ] {
            let mut harness = harness(
                Document {
                    language,
                    ..document()
                },
                egui::vec2(1440.0, 960.0),
            );
            harness.ctx.add_plugin(RepeatKeyPass);
            select_range(&mut harness);
            press_letter_order(&mut harness, Key::N, text, order);
            assert!(state(&harness).event_editor_open);
            let title = language.text("Придумайте название", "Add a title");
            assert!(field(&harness, title).is_focused());
            assert_eq!(field(&harness, title).value().as_deref(), Some(""));
            assert_eq!(
                field(&harness, language.text("Дата начала", "Start date"))
                    .value()
                    .as_deref(),
                Some(language.text("12.01.2026", "2026-01-12"))
            );
            assert_eq!(
                field(&harness, language.text("Дата окончания", "End date"))
                    .value()
                    .as_deref(),
                Some(language.text("14.01.2026", "2026-01-14"))
            );
            key(&mut harness, Key::Escape);
            assert!(!state(&harness).event_editor_open);
            assert_eq!(
                state(&harness).selection,
                Some(range("2026-01-12", "2026-01-14"))
            );
            key(&mut harness, Key::Escape);
            assert!(state(&harness).selection.is_none());
        }
    }
}

#[test]
fn today_shortcut_selects_today_and_jumps_in_both_calendar_modes_and_layouts() {
    for view_mode in [CalendarViewMode::SingleYear, CalendarViewMode::Continuous] {
        for text in ["t", "е"] {
            let mut document = document();
            let today = document.display_timezone.today();
            let today_year = testing::Year::try_from(today.year()).unwrap();
            document.year = today_year.step(-2).unwrap();
            document.view_mode = view_mode;
            let mut harness = harness(document, egui::vec2(1440.0, 960.0));
            press_letter(&mut harness, Key::T, text);
            assert_eq!(state(&harness).document.year, today_year);
            assert_eq!(
                state(&harness).selection,
                Some(testing::DateRange::between(today, today))
            );
            let today_label = today.to_string();
            let node = harness.get_by_label(&today_label);
            assert!(node.rect().intersects(harness.ctx.content_rect()));
        }
    }
}

#[test]
fn page_keys_switch_years_in_both_modes_and_respect_limits() {
    for view_mode in [CalendarViewMode::SingleYear, CalendarViewMode::Continuous] {
        let mut harness = harness(
            Document {
                view_mode,
                ..document()
            },
            egui::vec2(1440.0, 960.0),
        );
        select_range(&mut harness);
        key(&mut harness, Key::PageDown);
        assert_eq!(state(&harness).document.year.get(), 2027);
        assert!(state(&harness).selection.is_none());
        assert!(state(&harness).dirty);
        key(&mut harness, Key::PageUp);
        assert_eq!(state(&harness).document.year.get(), 2026);
        for (year, key_code) in [(1900, Key::PageUp), (2100, Key::PageDown)] {
            let mut harness = super::harness(
                Document {
                    view_mode,
                    year: testing::Year::try_from(year).unwrap(),
                    ..document()
                },
                egui::vec2(1440.0, 960.0),
            );
            key(&mut harness, key_code);
            assert_eq!(state(&harness).document.year.get(), year);
            assert!(!state(&harness).dirty);
        }
    }
}

#[test]
fn calendar_shortcuts_do_not_interrupt_search_text_or_filters() {
    for size in [egui::vec2(800.0, 700.0), egui::vec2(1440.0, 960.0)] {
        let mut harness = harness(document(), size);
        open_search(&mut harness);
        press_letter(&mut harness, Key::N, "т");
        press_letter(&mut harness, Key::T, "е");
        assert_eq!(state(&harness).search.query, "те");
        for key_code in [Key::PageUp, Key::PageDown] {
            key(&mut harness, key_code);
        }
        assert_eq!(state(&harness).document.year.get(), 2026);
        assert!(!state(&harness).event_editor_open);
        harness
            .get_by(|node| {
                node.role() == Role::ComboBox && node.value().as_deref() == Some("Все даты")
            })
            .click();
        harness.run();
        key(&mut harness, Key::Escape);
        assert!(!egui::Popup::is_any_open(&harness.ctx));
        assert!(state(&harness).search.open);
        key(&mut harness, Key::Escape);
        assert!(!state(&harness).search.open);
        assert_eq!(state(&harness).search.query, "те");
    }
}

#[test]
fn escape_closes_search_and_menus_before_clearing_selection() {
    let mut harness = harness(document(), egui::vec2(1440.0, 960.0));
    harness.ctx.add_plugin(RepeatKeyPass);
    select_range(&mut harness);
    open_search(&mut harness);
    key(&mut harness, Key::Escape);
    assert!(!state(&harness).search.open);
    assert!(state(&harness).selection.is_some());
    click(&mut harness, "Настройки");
    key(&mut harness, Key::N);
    key(&mut harness, Key::T);
    key(&mut harness, Key::PageDown);
    assert!(!state(&harness).event_editor_open);
    assert_eq!(state(&harness).document.year.get(), 2026);
    key(&mut harness, Key::Escape);
    assert!(!egui::Popup::is_any_open(&harness.ctx));
    assert!(state(&harness).selection.is_some());
    key(&mut harness, Key::Escape);
    assert!(state(&harness).selection.is_none());
}

#[test]
fn shortcuts_do_not_interrupt_year_or_event_editing() {
    let mut harness = harness(document(), egui::vec2(1440.0, 960.0));
    click(&mut harness, "2026");
    click(&mut harness, "2026");
    for key_code in [Key::N, Key::T, Key::PageUp, Key::PageDown] {
        key(&mut harness, key_code);
    }
    harness.key_press_modifiers(Modifiers::COMMAND, Key::K);
    harness.run();
    assert_eq!(field(&harness, "Год").value().as_deref(), Some("2026"));
    assert_eq!(state(&harness).document.year.get(), 2026);
    assert!(!state(&harness).event_editor_open);
    assert!(!state(&harness).search.open);
    key(&mut harness, Key::Escape);
    key(&mut harness, Key::N);
    press_letter(&mut harness, Key::N, "n");
    press_letter(&mut harness, Key::T, "t");
    assert_eq!(
        field(&harness, "Придумайте название").value().as_deref(),
        Some("nt")
    );
    for key_code in [Key::PageUp, Key::PageDown] {
        key(&mut harness, key_code);
    }
    harness.key_press_modifiers(Modifiers::COMMAND, Key::K);
    harness.run();
    assert_eq!(state(&harness).document.year.get(), 2026);
    assert!(!state(&harness).search.open);
    assert!(state(&harness).document.events.is_empty());
}

#[test]
fn save_shortcut_preserves_validation_and_saves_description_without_extra_newlines() {
    for language in [Language::Russian, Language::English] {
        let mut harness = harness(
            Document {
                language,
                ..document()
            },
            egui::vec2(1440.0, 960.0),
        );
        harness.ctx.add_plugin(RepeatKeyPass);
        key(&mut harness, Key::N);
        harness.key_press_modifiers(Modifiers::COMMAND, Key::Enter);
        harness.run();
        assert!(state(&harness).event_editor_open);
        assert_eq!(state(&harness).event_error, Some(InputError::EmptyTitle));
        assert!(state(&harness).document.events.is_empty());
        type_into(
            &mut harness,
            language.text("Придумайте название", "Add a title"),
            "Meeting",
        );
        description(&harness, language).scroll_to_me();
        harness.run();
        description(&harness, language).focus();
        harness.run();
        assert!(description(&harness, language).is_focused());
        description(&harness, language).type_text("First");
        harness.run();
        key(&mut harness, Key::Enter);
        assert!(state(&harness).event_editor_open);
        description(&harness, language).type_text("Second");
        harness.run();
        assert_eq!(
            description(&harness, language).value().as_deref(),
            Some("First\nSecond")
        );
        harness.key_press_modifiers(Modifiers::COMMAND, Key::Enter);
        harness.run();
        assert!(!state(&harness).event_editor_open);
        assert_eq!(state(&harness).document.events.len(), 1);
        let event = &state(&harness).document.events[0];
        assert_eq!(event.title.get(), "Meeting");
        assert_eq!(event.notes, "First\nSecond");
    }
}

#[test]
fn save_shortcut_is_blocked_by_the_event_calendar_popup() {
    let mut harness = harness(document(), egui::vec2(1440.0, 960.0));
    harness.ctx.add_plugin(RepeatKeyPass);
    key(&mut harness, Key::N);
    type_into(&mut harness, "Придумайте название", "Meeting");
    harness
        .get_by_role_and_label(Role::Window, "Событие")
        .get_by(|node| {
            node.role() == Role::ComboBox
                && node.value().is_some_and(|value| value.starts_with('●'))
        })
        .click();
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Enter);
    harness.run();
    assert!(state(&harness).event_editor_open);
    assert!(state(&harness).document.events.is_empty());
    key(&mut harness, Key::Escape);
    assert!(state(&harness).event_editor_open);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Enter);
    harness.run();
    assert!(!state(&harness).event_editor_open);
}

#[test]
fn extra_modifiers_and_repeated_presses_do_not_trigger_shortcuts() {
    let mut harness = harness(document(), egui::vec2(1440.0, 960.0));
    select_range(&mut harness);
    for modifiers in [Modifiers::SHIFT, Modifiers::ALT, Modifiers::COMMAND] {
        for key_code in [Key::N, Key::T, Key::PageUp, Key::PageDown, Key::Escape] {
            harness.key_press_modifiers(modifiers, key_code);
            harness.run();
        }
    }
    for key_code in [Key::N, Key::T, Key::PageUp, Key::PageDown, Key::Escape] {
        repeat_key(&mut harness, key_code);
    }
    assert!(!state(&harness).event_editor_open);
    assert_eq!(state(&harness).document.year.get(), 2026);
    assert_eq!(
        state(&harness).selection,
        Some(range("2026-01-12", "2026-01-14"))
    );
    key(&mut harness, Key::N);
    repeat_key(&mut harness, Key::Escape);
    assert!(state(&harness).event_editor_open);
}
