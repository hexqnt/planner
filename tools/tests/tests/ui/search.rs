use super::*;

#[test]
fn search_preserves_document_and_selection_until_navigation_and_escape_keeps_query() {
    let mut harness = harness(search_document(), egui::vec2(1440.0, 960.0));
    let before = document_json(&harness);
    open_search(&mut harness);
    type_into(&mut harness, "Поиск событий…", "план офис");
    harness.get_by_label("Найдено: 1");
    assert!(state(&harness).selection.is_none());
    assert!(!state(&harness).event_editor_open);
    assert_eq!(document_json(&harness), before);
    key(&mut harness, Key::Enter);
    assert_eq!(state(&harness).document.year.get(), 2030);
    assert_eq!(
        state(&harness).selection,
        Some(range("2030-10-12", "2030-10-14"))
    );
    assert!(!state(&harness).event_editor_open);
    key(&mut harness, Key::Escape);
    assert!(!state(&harness).search.open);
    assert_eq!(state(&harness).search.query, "план офис");
    open_search(&mut harness);
    assert_eq!(
        field(&harness, "Поиск событий…").value().as_deref(),
        Some("план офис")
    );
}

#[test]
fn arrow_navigation_preserves_calendar_and_enter_uses_the_selected_result() {
    let mut document = search_document();
    let mut other = event(
        2,
        document.events[0].category,
        EventSchedule::all_day(range("2031-01-10", "2031-01-10")),
    );
    other.title = Title::try_from("Планирование").unwrap();
    document.events.push(other);
    let mut harness = harness(document, egui::vec2(1440.0, 960.0));
    open_search(&mut harness);
    type_into(&mut harness, "Поиск событий…", "план");
    let before = document_json(&harness);
    key(&mut harness, Key::ArrowDown);
    assert_eq!(state(&harness).search.selected, 1);
    assert_eq!(document_json(&harness), before);
    assert!(state(&harness).selection.is_none());
    key(&mut harness, Key::Enter);
    assert_eq!(state(&harness).document.year.get(), 2031);
    assert_eq!(
        state(&harness).selection,
        Some(range("2031-01-10", "2031-01-10"))
    );
}

#[test]
fn narrow_search_closes_after_navigation_and_keeps_the_query_across_resize() {
    let mut harness = harness(search_document(), egui::vec2(820.0, 700.0));
    open_search(&mut harness);
    type_into(&mut harness, "Поиск событий…", "план");
    key(&mut harness, Key::Enter);
    assert!(!state(&harness).search.open);
    assert_eq!(state(&harness).document.year.get(), 2030);
    assert_eq!(state(&harness).search.query, "план");
    open_search(&mut harness);
    harness.set_size(egui::vec2(1440.0, 960.0));
    harness.run();
    assert!(state(&harness).search.open);
    assert_eq!(
        field(&harness, "Поиск событий…").value().as_deref(),
        Some("план")
    );
    harness.get_by_label("Найдено: 1");
}

#[test]
fn picking_current_year_captures_the_range_and_can_be_explicitly_refreshed() {
    let mut harness = harness(search_document(), egui::vec2(1440.0, 960.0));
    open_search(&mut harness);
    type_into(&mut harness, "Поиск событий…", "план");
    choose(&mut harness, "Все даты", "Текущий год");
    assert_eq!(
        state(&harness).search.dates,
        Some(range("2026-01-01", "2026-12-31"))
    );
    assert_eq!(state(&harness).search.results, 0);
    for _ in 0..4 {
        click(&mut harness, ">");
    }
    assert_eq!(state(&harness).document.year.get(), 2030);
    assert_eq!(
        state(&harness).search.dates,
        Some(range("2026-01-01", "2026-12-31"))
    );
    choose(&mut harness, "Год 2026", "Текущий год");
    assert_eq!(
        state(&harness).search.dates,
        Some(range("2030-01-01", "2030-12-31"))
    );
    assert_eq!(state(&harness).search.results, 1);
}

#[test]
fn editor_button_opens_the_source_event_without_navigating() {
    let mut harness = harness(search_document(), egui::vec2(1440.0, 960.0));
    let before = document_json(&harness);
    edit_first_result(&mut harness);
    assert_eq!(state(&harness).document.year.get(), 2026);
    assert!(state(&harness).selection.is_none());
    assert_eq!(
        field(&harness, "Придумайте название").value().as_deref(),
        Some("Планирование")
    );
    assert_eq!(document_json(&harness), before);
}

#[test]
fn clearing_the_query_restores_focus_and_does_not_change_the_document() {
    for language in [Language::Russian, Language::English] {
        for size in [egui::vec2(1440.0, 960.0), egui::vec2(360.0, 600.0)] {
            let mut document = search_document();
            document.language = language;
            let mut harness = harness(document, size);
            let before = document_json(&harness);
            let label = language.text("Поиск событий…", "Search events…");
            open_search(&mut harness);
            type_into(&mut harness, label, "несуществующее событие");
            harness.get_by_label(language.text("Найдено: 0", "Found: 0"));
            click(
                &mut harness,
                language.text("Очистить запрос", "Clear query"),
            );
            assert_eq!(state(&harness).search.query, "");
            assert!(field(&harness, label).is_focused());
            field(&harness, label).type_text("план");
            harness.run();
            harness.get_by_label(language.text("Найдено: 1", "Found: 1"));
            assert_eq!(document_json(&harness), before);
        }
    }
}

#[test]
fn removing_filter_chips_preserves_the_query_and_other_filters() {
    for language in [Language::Russian, Language::English] {
        let mut document = search_document();
        document.language = language;
        let mut harness = harness(document, egui::vec2(1440.0, 960.0));
        let before = document_json(&harness);
        open_search(&mut harness);
        type_into(
            &mut harness,
            language.text("Поиск событий…", "Search events…"),
            "план",
        );
        choose(
            &mut harness,
            language.text("Все даты", "All dates"),
            language.text("Текущий год", "Current year"),
        );
        click(&mut harness, language.text("Фильтры", "Filters"));
        choose(
            &mut harness,
            language.text("Любой статус", "Any status"),
            language.text("Подтверждено", "Confirmed"),
        );
        assert_eq!(state(&harness).search.results, 0);
        click(&mut harness, language.text("Подтверждено ×", "Confirmed ×"));
        assert_eq!(
            state(&harness).search.dates,
            Some(range("2026-01-01", "2026-12-31"))
        );
        assert_eq!(state(&harness).search.results, 0);
        click(&mut harness, "01.01.2026 — 31.12.2026 ×");
        assert!(state(&harness).search.dates.is_none());
        assert_eq!(state(&harness).search.results, 1);
        replace_text(
            &mut harness,
            language.text("Начало с", "Starts at or after"),
            "10:00",
        );
        assert_eq!(state(&harness).search.results, 0);
        click(&mut harness, language.text("С 10:00 ×", "From 10:00 ×"));
        assert_eq!(
            field(&harness, language.text("Начало с", "Starts at or after"))
                .value()
                .as_deref(),
            Some("")
        );
        assert_eq!(state(&harness).search.results, 1);
        assert_eq!(state(&harness).search.query, "план");
        assert_eq!(document_json(&harness), before);
    }
}

#[test]
fn calendar_picker_filters_by_scope_and_individual_calendar_without_editing_the_document() {
    for language in [Language::Russian, Language::English] {
        let mut document = search_document();
        document.language = language;
        document.groups[0].enabled = false;
        let first = document.groups[0].categories[0]
            .name
            .get(language)
            .to_owned();
        let other = document.groups[0].categories[1]
            .name
            .get(language)
            .to_owned();
        let mut harness = harness(document, egui::vec2(1440.0, 960.0));
        let saved = document_json(&harness);
        open_search(&mut harness);
        type_into(
            &mut harness,
            language.text("Поиск событий…", "Search events…"),
            "план",
        );
        assert_eq!(state(&harness).search.results, 1);
        choose(
            &mut harness,
            language.text("Все календари", "All calendars"),
            language.text("Только видимые", "Visible calendars only"),
        );
        assert_eq!(state(&harness).search.results, 0);
        choose(
            &mut harness,
            language.text("Только видимые", "Visible calendars only"),
            language.text("Все календари", "All calendars"),
        );
        harness
            .get_by(|node| {
                node.role() == Role::ComboBox
                    && node.value().as_deref()
                        == Some(language.text("Все календари", "All calendars"))
            })
            .click();
        harness.run();
        harness.get_by_role_and_label(Role::Button, &first).click();
        harness.run();
        assert_eq!(state(&harness).search.results, 1);
        harness
            .get_by(|node| {
                node.role() == Role::ComboBox
                    && node.value().is_some_and(|value| value.contains(&first))
            })
            .click();
        harness.run();
        harness.get_by_role_and_label(Role::Button, &other).click();
        harness.run();
        assert_eq!(state(&harness).search.results, 0);
        assert_eq!(document_json(&harness), saved);
    }
}
