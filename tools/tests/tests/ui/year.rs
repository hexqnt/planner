use super::*;
use egui_kittest::kittest::NodeT as _;

fn edit_year(harness: &mut AppHarness, language: Language) {
    click(harness, "2026");
    assert!(
        harness
            .query_by_role_and_label(Role::TextInput, language.text("Год", "Year"))
            .is_none()
    );
    click(harness, "2026");
    assert!(field(harness, language.text("Год", "Year")).is_focused());
}

#[test]
fn year_input_filters_digits_and_accepts_cancels_or_commits_on_blur() {
    for language in [Language::Russian, Language::English] {
        for (text, finish, expected) in [
            ("20ab30", Some(Key::Enter), 2030),
            ("", Some(Key::Enter), 2026),
            ("2030", Some(Key::Escape), 2026),
            ("2031", None, 2031),
            ("1", Some(Key::Enter), 1900),
            ("9999999999999999999999", Some(Key::Enter), 2100),
        ] {
            let mut harness = harness(
                Document {
                    language,
                    ..document()
                },
                egui::vec2(1440.0, 960.0),
            );
            edit_year(&mut harness, language);
            let label = language.text("Год", "Year");
            harness
                .input_mut()
                .events
                .push(egui::Event::Paste(text.into()));
            if text.is_empty() {
                harness.key_press(Key::Backspace);
            }
            harness.run();
            assert_eq!(
                field(&harness, label).value().unwrap(),
                text.chars()
                    .filter(char::is_ascii_digit)
                    .collect::<String>()
            );
            if let Some(finish) = finish {
                key(&mut harness, finish);
            } else {
                click(&mut harness, language.text("Настройки", "Settings"));
            }
            assert!(
                harness
                    .query_by_role_and_label(Role::TextInput, label)
                    .is_none()
            );
            assert_eq!(state(&harness).document.year.get(), expected);
            assert_eq!(state(&harness).dirty, expected != 2026);
        }
    }
}

fn wheel(harness: &mut AppHarness, pointer: egui::Pos2, delta: egui::Vec2) {
    harness.input_mut().events.extend([
        egui::Event::PointerMoved(pointer),
        egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta,
            phase: egui::TouchPhase::Move,
            modifiers: Modifiers::NONE,
        },
    ]);
    harness.run_steps(40);
}

#[test]
fn year_wheel_requires_hover_and_ignores_horizontal_scroll_and_active_editing() {
    let mut harness = harness(document(), egui::vec2(1440.0, 960.0));
    let pointer = harness.get_by_label("2026").rect().center();
    wheel(&mut harness, pointer, egui::Vec2::Y);
    assert_eq!(state(&harness).document.year.get(), 2027);
    wheel(&mut harness, pointer, -egui::Vec2::Y);
    assert_eq!(state(&harness).document.year.get(), 2026);
    wheel(&mut harness, pointer, egui::Vec2::X);
    assert_eq!(state(&harness).document.year.get(), 2026);
    wheel(&mut harness, egui::pos2(5.0, 5.0), egui::Vec2::Y);
    assert_eq!(state(&harness).document.year.get(), 2026);
    edit_year(&mut harness, Language::Russian);
    let pointer = field(&harness, "Год").rect().center();
    wheel(&mut harness, pointer, egui::Vec2::Y);
    assert_eq!(field(&harness, "Год").value().as_deref(), Some("2026"));
    assert_eq!(state(&harness).document.year.get(), 2026);
    key(&mut harness, Key::Escape);
}

#[test]
fn year_wheel_respects_both_limits() {
    for (year, delta, hidden_arrow) in [(1900, -egui::Vec2::Y, "<"), (2100, egui::Vec2::Y, ">")] {
        let mut harness = harness(
            Document {
                year: testing::Year::try_from(year).unwrap(),
                ..document()
            },
            egui::vec2(1440.0, 960.0),
        );
        let pointer = harness.get_by_label(&year.to_string()).rect().center();
        wheel(&mut harness, pointer, delta);
        assert_eq!(state(&harness).document.year.get(), year);
        assert!(!state(&harness).dirty);
        assert!(
            harness
                .get_by_role_and_label(Role::Button, hidden_arrow)
                .accesskit_node()
                .is_disabled()
        );
    }
}
