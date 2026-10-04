use super::*;

fn toggle_vacation(harness: &mut AppHarness) {
    let language = state(harness).document.language;
    let enabled = state(harness).document.vacation.enabled;
    assert!(
        harness
            .query_by_label(language.text("Режим отпуска", "Vacation mode"))
            .is_none()
    );
    click(harness, language.text("Настройки", "Settings"));
    let checkbox = harness.get_by_role_and_label(
        Role::CheckBox,
        language.text("Режим отпуска", "Vacation mode"),
    );
    checkbox.click();
    harness.run();
    assert_eq!(state(harness).document.vacation.enabled, !enabled);
    assert!(!egui::Popup::is_any_open(&harness.ctx));
}

fn open_calculation(harness: &mut AppHarness) {
    if !state(harness).document.vacation.enabled {
        toggle_vacation(harness);
    }
    assert!(state(harness).vacation.open);
}

fn close_calculation(harness: &mut AppHarness) {
    let label = state(harness).document.language.text("Закрыть", "Close");
    click(harness, label);
    assert!(!state(harness).vacation.open);
    assert!(!state(harness).document.vacation.enabled);
}

#[test]
fn calculation_header_centers_icons_and_wrapped_titles() {
    for language in [Language::Russian, Language::English] {
        for size in [egui::vec2(1440.0, 960.0), egui::vec2(360.0, 600.0)] {
            let mut harness = harness(
                Document {
                    language,
                    ..document()
                },
                size,
            );
            open_calculation(&mut harness);
            let title = language.text("Расчёт отпуска", "Vacation calculation");
            let window = harness.get_by_role_and_label(Role::Window, title);
            let center = window
                .get_by_role_and_label(Role::Label, title)
                .rect()
                .center()
                .y;
            for rect in [
                window
                    .get_by_role_and_label(Role::Image, language.text("Калькулятор", "Calculator"))
                    .rect(),
                window
                    .get_by_role_and_label(
                        Role::Image,
                        language.text("О расчёте отпуска", "About vacation calculation"),
                    )
                    .rect(),
                window.get_by_label("×").rect(),
            ] {
                assert!(
                    (rect.center().y - center).abs() <= 0.5,
                    "{title}, {size:?}: {rect:?}, title center {center}"
                );
            }
        }
    }
}

#[test]
fn calculation_help_is_next_to_the_relevant_fields_and_fits_small_screens() {
    for language in [Language::Russian, Language::English] {
        for size in [egui::vec2(1440.0, 960.0), egui::vec2(360.0, 600.0)] {
            let mut harness = harness(
                Document {
                    language,
                    ..document()
                },
                size,
            );
            open_calculation(&mut harness);
            for (label, text) in [
                (language.text("О расчёте отпуска", "About vacation calculation"), language.text("Россия · пятидневка · до НДФЛ · приблизительный расчёт", "Russia · five-day workweek · before tax · estimate")),
                (language.text("Даты отпуска", "Vacation dates"), language.text("Выделите даты отпуска на календаре: перетаскиванием или Shift + кликом. Под датой — изменение дохода при включении этого дня в отпуск. Включённые выходные тоже расходуют дни отпуска, праздничные дни исключаются. Потеря зарплаты зависит от нормы рабочих дней каждого месяца.", "Select vacation dates on the calendar by dragging or Shift-clicking. Each date shows the income change if that day is included in leave. Included weekends also use leave days; public holidays are excluded. Salary reduction depends on each month's workday norm.")),
                (language.text("Оклад, ₽/мес", "Monthly salary, RUB"), language.text("Оценка по окладу использует оклад / 29,3 и предполагает постоянный заработок и полностью отработанный расчётный период. Известную ставку можно взять из расчёта бухгалтерии.", "The salary-based estimate uses salary / 29.3 and assumes constant earnings and a fully worked reference period. A known daily rate can be taken from payroll.")),
            ] {
                let target = if label == language.text("О расчёте отпуска", "About vacation calculation") {
                    harness.get_by_role_and_label(Role::Image, label).rect()
                } else {
                    harness.get_by_role_and_label(Role::Label, label).rect()
                };
                super::help::check_hint(&mut harness, target, text);
                assert!(state(&harness).vacation.open);
            }
            close_calculation(&mut harness);
        }
    }
}

fn select(harness: &mut AppHarness, start: &str, end: &str) {
    let reopen = state(harness).vacation.open;
    if reopen {
        close_calculation(harness);
    }
    click(harness, start);
    if start != end {
        harness.get_by_label(end).click_modifiers(Modifiers::SHIFT);
        harness.run();
    }
    assert_eq!(state(harness).selection, Some(range(start, end)));
    if reopen {
        open_calculation(harness);
    }
}

#[test]
fn metrics_without_selected_dates_show_zero_changes_and_full_salary() {
    for language in [Language::Russian, Language::English] {
        for mode in [CalendarViewMode::SingleYear, CalendarViewMode::Continuous] {
            let mut harness = harness(
                Document {
                    language,
                    view_mode: mode,
                    ..document()
                },
                egui::vec2(1440.0, 960.0),
            );
            open_calculation(&mut harness);
            for (russian, english) in [
                ("Отпускные", "Vacation pay"),
                ("Потеря зарплаты", "Salary reduction"),
                ("Оставшаяся зарплата", "Remaining salary"),
                ("Изменение дохода", "Income change"),
            ] {
                harness.get_by_role_and_label(Role::Label, language.text(russian, english));
            }
            assert!(state(&harness).selection.is_none());
            assert!(state(&harness).vacation.estimate.is_none());
            assert_eq!(
                harness
                    .get_all_by_label(language.text("0,00 ₽", "0.00 ₽"))
                    .count(),
                3
            );
            harness.get_by_label(language.text("40000,00 ₽", "40000.00 ₽"));
            assert!(
                harness
                    .query_by_label(
                        language.text("Создать событие «Отпуск»", "Create vacation event")
                    )
                    .is_none()
            );

            let salary_label = language.text("Оклад, ₽/мес", "Monthly salary, RUB");
            replace_text(&mut harness, salary_label, "80000");
            harness.get_by_label(language.text("80000,00 ₽", "80000.00 ₽"));
            assert!(
                harness
                    .query_by_label(language.text("40000,00 ₽", "40000.00 ₽"))
                    .is_none()
            );
            replace_text(&mut harness, salary_label, "invalid");
            assert_eq!(harness.get_all_by_label("—").count(), 4);
            assert!(
                harness
                    .query_by_label(language.text("80000,00 ₽", "80000.00 ₽"))
                    .is_none()
            );
            replace_text(&mut harness, salary_label, "80000");
            harness.get_by_label(language.text("80000,00 ₽", "80000.00 ₽"));
            select(&mut harness, "2026-10-03", "2026-10-09");
            harness.get_by_label(language.text("19112,63 ₽", "19112.63 ₽"));
            harness.get_by_label(language.text("+930,81 ₽", "+930.81 ₽"));
        }
    }
}

#[test]
fn vacation_mode_selects_dates_displays_estimate_and_creates_event() {
    for language in [Language::Russian, Language::English] {
        for mode in [CalendarViewMode::SingleYear, CalendarViewMode::Continuous] {
            let mut harness = harness(
                Document {
                    language,
                    view_mode: mode,
                    ..document()
                },
                egui::vec2(3000.0, 1500.0),
            );
            toggle_vacation(&mut harness);
            assert!(state(&harness).document.vacation.enabled);
            select(&mut harness, "2026-10-03", "2026-10-09");
            assert!(state(&harness).vacation.estimate.is_some());
            harness.get_by_label(language.text("9556,31 ₽", "9556.31 ₽"));
            harness.get_by_label(language.text("+465,40 ₽", "+465.40 ₽"));
            click(
                &mut harness,
                language.text("Создать событие «Отпуск»", "Create vacation event"),
            );
            assert!(state(&harness).event_editor_open);
            assert!(!state(&harness).vacation.open);
            assert!(!state(&harness).document.vacation.enabled);
            assert_eq!(
                field(
                    &harness,
                    language.text("Придумайте название", "Add a title")
                )
                .value()
                .as_deref(),
                Some(language.text("Отпуск", "Vacation"))
            );
            click(&mut harness, language.text("Сохранить", "Save"));
            assert!(!state(&harness).event_editor_open);
            let snapshot = state(&harness);
            let event = &snapshot.document.events[0];
            assert_eq!(event.schedule.dates(), range("2026-10-03", "2026-10-09"));
            assert!(event.schedule.times().is_none());
            assert_eq!(
                snapshot
                    .document
                    .category(event.category)
                    .unwrap()
                    .name
                    .get(language),
                language.text("Отпуск", "Vacation")
            );
            assert!(snapshot.dirty);
            assert!(!state(&harness).document.vacation.enabled);
            assert!(state(&harness).vacation.estimate.is_none());
            assert!(
                harness
                    .query_by_label(language.text("Оклад, ₽/мес", "Monthly salary, RUB"))
                    .is_none()
            );
        }
    }
}

#[test]
fn vacation_creation_survives_repeated_layout_passes() {
    let mut harness = harness(document(), egui::vec2(1440.0, 960.0));
    select(&mut harness, "2026-10-03", "2026-10-09");
    open_calculation(&mut harness);
    harness
        .ctx
        .add_plugin(super::dialogs::RepeatedPass::default());
    click(&mut harness, "Создать событие «Отпуск»");
    assert!(state(&harness).event_editor_open);
    assert!(!state(&harness).vacation.open);
    assert_eq!(
        field(&harness, "Придумайте название").value().as_deref(),
        Some("Отпуск")
    );
    click(&mut harness, "Сохранить");
    assert_eq!(state(&harness).document.events.len(), 1);
    assert_eq!(
        state(&harness).document.events[0].schedule.dates(),
        range("2026-10-03", "2026-10-09")
    );
}

#[test]
fn editing_money_recalculates_and_invalid_input_hides_stale_results() {
    let mut document = document();
    document.vacation.enabled = true;
    let mut harness = harness(document, egui::vec2(3000.0, 1500.0));
    open_calculation(&mut harness);
    select(&mut harness, "2026-10-03", "2026-10-09");
    replace_text(&mut harness, "Оклад, ₽/мес", "80 000,00");
    harness.get_by_label("+930,81 ₽");
    click(&mut harness, "Указать средний дневной заработок");
    replace_text(&mut harness, "Средний заработок, ₽/день", "3000,25");
    harness.get_by_label("21001,75 ₽");
    let saved = document_json(&harness);
    replace_text(&mut harness, "Средний заработок, ₽/день", "-1");
    assert!(state(&harness).vacation.estimate.is_none());
    assert_eq!(harness.get_all_by_label("—").count(), 4);
    assert!(harness.query_by_label("21001,75 ₽").is_none());
    assert!(harness.query_by_label("Создать событие «Отпуск»").is_none());
    assert_eq!(document_json(&harness), saved);
    replace_text(&mut harness, "Средний заработок, ₽/день", "3000,25");
    harness.get_by_label("21001,75 ₽");
    let restored = Document::parse(&document_json(&harness).to_string()).unwrap();
    let mut restored = planner_test_support::harness(restored, egui::vec2(3000.0, 1500.0));
    open_calculation(&mut restored);
    assert_eq!(
        field(&restored, "Оклад, ₽/мес").value().as_deref(),
        Some("80000,00")
    );
    assert_eq!(
        field(&restored, "Средний заработок, ₽/день")
            .value()
            .as_deref(),
        Some("3000,25")
    );
    select(&mut restored, "2026-10-03", "2026-10-09");
    restored.get_by_label("21001,75 ₽");
}

#[test]
fn changing_region_recalculates_holidays_and_cross_year_selection_survives_geometry_changes() {
    let mut document = document();
    document.view_mode = CalendarViewMode::Continuous;
    document.vacation.enabled = true;
    let mut harness = harness(document, egui::vec2(3000.0, 1800.0));
    open_calculation(&mut harness);
    select(&mut harness, "2026-11-06", "2026-11-06");
    assert_eq!(state(&harness).vacation.estimate.unwrap().paid_days, 1);
    choose(&mut harness, "Федеральный", "Татарстан");
    assert_eq!(state(&harness).vacation.estimate.unwrap().paid_days, 0);
    select(&mut harness, "2026-12-30", "2027-01-03");
    let selected = state(&harness).selection;
    toggle_vacation(&mut harness);
    assert_eq!(state(&harness).selection, selected);
    toggle_vacation(&mut harness);
    assert_eq!(state(&harness).selection, selected);
    assert!(state(&harness).vacation.estimate.is_some());
    harness.get_by_label("По месяцам");
}

#[test]
fn vacation_window_fits_small_screens_in_both_languages_and_themes() {
    for language in [Language::Russian, Language::English] {
        for dark in [false, true] {
            for size in [egui::vec2(360.0, 600.0), egui::vec2(760.0, 600.0)] {
                let mut document = Document {
                    language,
                    dark,
                    ..document()
                };
                document.vacation.enabled = true;
                let mut harness = harness(document, size);
                open_calculation(&mut harness);
                let label = language.text("Оклад, ₽/мес", "Monthly salary, RUB");
                let bounds = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
                assert!(bounds.contains_rect(harness.get_by_label("×").rect()));
                assert!(
                    bounds.contains_rect(
                        harness
                            .get_by_role_and_label(
                                Role::Label,
                                language.text("Расчёт отпуска", "Vacation calculation")
                            )
                            .rect()
                    )
                );
                let window = testing::vacation_window_rect(&harness.ctx).unwrap();
                assert!(bounds.contains_rect(window));
                let salary = field(&harness, label).rect();
                assert!(window.contains_rect(salary));
                let salary_label = harness.get_by_role_and_label(Role::Label, label).rect();
                assert!(window.contains_rect(salary_label));
                assert!(salary_label.right() <= salary.left());
                assert!((salary_label.center().y - salary.center().y).abs() <= 0.5);
                let manual_label = language.text(
                    "Указать средний дневной заработок",
                    "Enter average daily earnings",
                );
                let checkbox = harness
                    .get_by_role_and_label(Role::CheckBox, manual_label)
                    .rect();
                click(&mut harness, manual_label);
                assert!(
                    bounds.contains_rect(
                        field(
                            &harness,
                            language.text("Средний заработок, ₽/день", "Average earnings, RUB/day")
                        )
                        .rect()
                    )
                );
                let average = field(
                    &harness,
                    language.text("Средний заработок, ₽/день", "Average earnings, RUB/day"),
                )
                .rect();
                assert!((average.left() - checkbox.left()).abs() <= 1.0);
                assert!(average.top() > salary.bottom());
                close_calculation(&mut harness);
                click(&mut harness, "☰");
                let first = harness.get_by_label("2026-01-12").rect().center();
                let last = harness.get_by_label("2026-01-16").rect().center();
                harness.event(egui::Event::PointerMoved(first));
                harness.event(egui::Event::PointerButton {
                    pos: first,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Modifiers::NONE,
                });
                harness.run_steps(2);
                harness.event(egui::Event::PointerMoved(first + egui::vec2(5.0, 0.0)));
                harness.run_steps(2);
                harness.event(egui::Event::PointerMoved(last));
                harness.run_steps(2);
                harness.event(egui::Event::PointerButton {
                    pos: last,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: Modifiers::NONE,
                });
                harness.event(egui::Event::PointerMoved(egui::pos2(0.0, 0.0)));
                harness.run();
                assert_eq!(
                    state(&harness).selection,
                    Some(range("2026-01-12", "2026-01-16"))
                );
                open_calculation(&mut harness);
                assert!(
                    bounds.contains_rect(
                        harness
                            .get_by_label(
                                language.text("Создать событие «Отпуск»", "Create vacation event")
                            )
                            .rect()
                    )
                );
            }
        }
    }
}

#[test]
fn closing_calculation_restores_normal_mode_and_reopening_preserves_selection_and_input() {
    for language in [Language::Russian, Language::English] {
        for mode in [CalendarViewMode::SingleYear, CalendarViewMode::Continuous] {
            for close in ["escape", "button", "header"] {
                let mut document = Document {
                    language,
                    view_mode: mode,
                    ..document()
                };
                document.vacation.enabled = true;
                let mut harness = harness(document, egui::vec2(1400.0, 900.0));
                let footer = harness.get_by_role_and_label(
                    Role::Pane,
                    language.text("Действия календаря", "Calendar actions"),
                );
                assert!(
                    footer
                        .query_by_role_and_label(
                            Role::Button,
                            language.text("Расчёт отпуска", "Vacation calculation")
                        )
                        .is_none()
                );
                select(&mut harness, "2026-01-12", "2026-01-16");
                replace_text(
                    &mut harness,
                    language.text("Оклад, ₽/мес", "Monthly salary, RUB"),
                    "80000",
                );
                let saved = document_json(&harness);
                let selection = state(&harness).selection;
                match close {
                    "escape" => key(&mut harness, Key::Escape),
                    "button" => close_calculation(&mut harness),
                    _ => click(&mut harness, "×"),
                }
                assert!(!state(&harness).vacation.open);
                assert!(!state(&harness).document.vacation.enabled);
                assert!(state(&harness).vacation.estimate.is_none());
                assert!(state(&harness).dirty);
                let mut expected = saved.clone();
                expected["vacation"]["enabled"] = false.into();
                assert_eq!(document_json(&harness), expected);
                assert_eq!(state(&harness).selection, selection);
                assert!(
                    harness
                        .query_by_label(language.text("Оклад, ₽/мес", "Monthly salary, RUB"))
                        .is_none()
                );
                open_calculation(&mut harness);
                assert_eq!(
                    field(
                        &harness,
                        language.text("Оклад, ₽/мес", "Monthly salary, RUB")
                    )
                    .value()
                    .as_deref(),
                    Some(language.text("80000,00", "80000.00"))
                );
                assert_eq!(state(&harness).selection, selection);
                assert!(state(&harness).vacation.estimate.is_some());
                assert_eq!(document_json(&harness), saved);
            }
        }
    }
}

#[test]
fn floating_calculation_moves_and_calendar_selection_remains_interactive() {
    let mut document = document();
    document.vacation.enabled = true;
    let mut harness = harness(document, egui::vec2(1400.0, 900.0));
    open_calculation(&mut harness);
    let before = testing::vacation_window_rect(&harness.ctx).unwrap();
    let start = before.left_top() + egui::vec2(120.0, 14.0);
    let end = start + egui::vec2(-280.0, 190.0);
    harness.event(egui::Event::PointerMoved(start));
    harness.event(egui::Event::PointerButton {
        pos: start,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    harness.run_steps(2);
    harness.event(egui::Event::PointerMoved(end));
    harness.run_steps(2);
    harness.event(egui::Event::PointerButton {
        pos: end,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.run();
    let after = testing::vacation_window_rect(&harness.ctx).unwrap();
    assert!(after.left() < before.left() - 200.0);
    assert!(after.top() > before.top() + 100.0);
    for date in ["2026-01-19", "2026-01-23"] {
        let rect = harness.get_by_label(date).rect();
        assert!(
            !after.intersects(rect),
            "Calendar date is exposed: {date} at {rect:?}, window {after:?}"
        );
    }
    click(&mut harness, "2026-01-19");
    assert_eq!(
        state(&harness).selection,
        Some(range("2026-01-19", "2026-01-19"))
    );
    let window = testing::vacation_window_rect(&harness.ctx).unwrap();
    assert!(
        (window.height() - after.height()).abs() <= 1.0,
        "Selection keeps the scroll viewport stable: {after:?} -> {window:?}"
    );
    harness
        .get_by_label("2026-01-23")
        .click_modifiers(Modifiers::SHIFT);
    harness.run();
    assert!(state(&harness).vacation.open);
    assert_eq!(
        state(&harness).selection,
        Some(range("2026-01-19", "2026-01-23"))
    );
    assert_eq!(state(&harness).vacation.estimate.unwrap().paid_days, 5);
}

#[test]
fn money_inputs_normalize_on_blur_and_report_each_invalid_field() {
    for language in [Language::Russian, Language::English] {
        let mut document = document();
        document.language = language;
        document.vacation.enabled = true;
        let mut harness = harness(document, egui::vec2(1440.0, 960.0));
        open_calculation(&mut harness);
        let salary = language.text("Оклад, ₽/мес", "Monthly salary, RUB");
        let average = language.text("Средний заработок, ₽/день", "Average earnings, RUB/day");
        click(
            &mut harness,
            language.text(
                "Указать средний дневной заработок",
                "Enter average daily earnings",
            ),
        );
        replace_text(&mut harness, salary, "80 000.5");
        focus_field(&mut harness, average);
        assert_eq!(
            field(&harness, salary).value().as_deref(),
            Some(language.text("80000,50", "80000.50"))
        );
        assert_eq!(state(&harness).document.vacation.salary.cents(), 8_000_050);
        replace_text(&mut harness, average, "1234,05");
        focus_field(&mut harness, salary);
        assert_eq!(
            field(&harness, average).value().as_deref(),
            Some(language.text("1234,05", "1234.05"))
        );
        let saved = document_json(&harness);
        replace_text(&mut harness, salary, "0");
        focus_field(&mut harness, average);
        replace_text(&mut harness, average, "invalid");
        focus_field(&mut harness, salary);
        let error = language.text(
            "Введите положительную сумму до 42 949 672,95 ₽, не более двух знаков после запятой.",
            "Enter a positive amount up to RUB 42,949,672.95 with at most two decimal places.",
        );
        assert_eq!(harness.get_all_by_label(error).count(), 2);
        assert_eq!(document_json(&harness), saved);
        replace_text(&mut harness, salary, "80000.50");
        replace_text(&mut harness, average, "1234.05");
        focus_field(&mut harness, salary);
        assert!(harness.query_by_label(error).is_none());
        assert_eq!(document_json(&harness), saved);
    }
}
