use super::*;
use crate::model::{Document, Year};

fn frame(
    ctx: &egui::Context,
    planner: &mut Planner,
    events: Vec<egui::Event>,
) -> (egui::FullOutput, egui::Pos2) {
    let mut button = egui::Pos2::ZERO;
    let output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(800.0, 800.0),
            )),
            events,
            ..Default::default()
        },
        |ui| {
            button = planner.timezone_button(ui).rect.center();
        },
    );
    (output, button)
}

fn click(ctx: &egui::Context, planner: &mut Planner, pos: egui::Pos2) {
    for pressed in [true, false] {
        frame(
            ctx,
            planner,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        )
        .0
        .drop_without_applying_deltas();
    }
}

fn text_center(output: &egui::FullOutput, label: &str) -> egui::Pos2 {
    crate::test_support::text_rect(output, label).center()
}

#[test]
fn timezone_search_selects_a_zone_and_quick_choices_restore_system_and_utc() {
    for language in [Language::Russian, Language::English] {
        let ctx = egui::Context::default();
        let mut planner = Planner::from_document(
            Document {
                language,
                ..Document::default()
            },
            &ctx,
            None,
        );
        let (output, button) = frame(&ctx, &mut planner, vec![]);
        output.drop_without_applying_deltas();
        click(&ctx, &mut planner, button);
        frame(&ctx, &mut planner, vec![])
            .0
            .drop_without_applying_deltas();
        frame(&ctx, &mut planner, vec![egui::Event::Text("moscow".into())])
            .0
            .drop_without_applying_deltas();
        assert_eq!(planner.timezone_picker.search.query, "moscow");
        assert!(egui::Popup::is_any_open(&ctx));
        let (output, _) = frame(&ctx, &mut planner, vec![]);
        let zone = text_center(&output, "Europe/Moscow");
        output.drop_without_applying_deltas();
        click(&ctx, &mut planner, zone);
        assert_eq!(
            planner.document.display_timezone,
            DisplayTimeZone::Named(chrono_tz::Europe::Moscow)
        );
        assert!(planner.persistence.is_dirty());
        assert!(!egui::Popup::is_any_open(&ctx));
        assert_eq!(
            planner.document.recent_timezones.iter().collect::<Vec<_>>(),
            [chrono_tz::Europe::Moscow]
        );
        for (label, expected) in [
            ("UTC", DisplayTimeZone::Utc),
            (
                language.text("Системная", "System"),
                DisplayTimeZone::System,
            ),
        ] {
            let (output, button) = frame(&ctx, &mut planner, vec![]);
            output.drop_without_applying_deltas();
            click(&ctx, &mut planner, button);
            let (output, _) = frame(&ctx, &mut planner, vec![]);
            assert_eq!(planner.timezone_picker.search.query, "");
            let target = text_center(&output, label);
            output.drop_without_applying_deltas();
            click(&ctx, &mut planner, target);
            assert_eq!(planner.document.display_timezone, expected);
        }
    }
}

#[test]
fn timezone_button_moves_to_a_second_row_on_small_screens_without_covering_the_year() {
    for width in [800.0, 1200.0] {
        let ctx = egui::Context::default();
        let mut planner = Planner::from_document(
            Document {
                year: Year::try_from(2026).unwrap(),
                ..Document::default()
            },
            &ctx,
            None,
        );
        let mut year = egui::Pos2::ZERO;
        let mut zone = egui::Pos2::ZERO;
        for _ in 0..2 {
            let output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(width, 900.0),
                    )),
                    ..Default::default()
                },
                |ui| planner.render(ui),
            );
            year = text_center(&output, "2026");
            zone = text_center(&output, "Системная");
            output.drop_without_applying_deltas();
        }
        if width < 1000.0 {
            assert!(zone.y > year.y + 20.0);
        } else {
            assert!((zone.y - year.y).abs() < 10.0);
            assert!(zone.x < year.x - 100.0);
        }
    }
}

#[test]
fn resizing_the_header_keeps_the_timezone_search_open_and_selectable() {
    let ctx = egui::Context::default();
    let mut planner = Planner::from_document(Document::default(), &ctx, None);
    let render = |planner: &mut Planner, width, events| {
        ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width, 900.0),
                )),
                events,
                ..Default::default()
            },
            |ui| planner.render(ui),
        )
    };
    render(&mut planner, 1200.0, vec![]).drop_without_applying_deltas();
    let output = render(&mut planner, 1200.0, vec![]);
    let pos = text_center(&output, "Системная");
    output.drop_without_applying_deltas();
    for pressed in [true, false] {
        render(
            &mut planner,
            1200.0,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        )
        .drop_without_applying_deltas();
    }
    render(&mut planner, 1200.0, vec![]).drop_without_applying_deltas();
    render(
        &mut planner,
        1200.0,
        vec![egui::Event::Text("moscow".into())],
    )
    .drop_without_applying_deltas();
    for width in [800.0, 1200.0, 800.0] {
        let output = render(&mut planner, width, vec![]);
        assert!(egui::Popup::is_any_open(&ctx));
        assert_eq!(planner.timezone_picker.search.query, "moscow");
        text_center(&output, "Europe/Moscow");
        output.drop_without_applying_deltas();
    }
    let output = render(&mut planner, 800.0, vec![]);
    let pos = text_center(&output, "Europe/Moscow");
    output.drop_without_applying_deltas();
    for pressed in [true, false] {
        render(
            &mut planner,
            800.0,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        )
        .drop_without_applying_deltas();
    }
    assert_eq!(
        planner.document.display_timezone,
        DisplayTimeZone::Named(chrono_tz::Europe::Moscow)
    );
}

#[test]
fn searching_zones_accepts_city_words_without_allocating_normalized_names() {
    assert!(matches_search("America/New_York", "new YORK"));
    assert!(matches_search("Europe/Moscow", "mosc"));
    assert!(matches_search("Asia/Tokyo", " "));
    assert!(!matches_search("Europe/Moscow", "tokyo"));
}

#[test]
fn today_navigation_uses_the_display_zone() {
    let ctx = egui::Context::default();
    let zone = DisplayTimeZone::Named(chrono_tz::Pacific::Kiritimati);
    let mut planner = Planner::from_document(
        Document {
            display_timezone: zone,
            year: Year::try_from(2000).unwrap(),
            ..Document::default()
        },
        &ctx,
        None,
    );
    let render = |planner: &mut Planner, events| {
        ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1000.0, 200.0),
                )),
                events,
                ..Default::default()
            },
            |ui| planner.footer(ui),
        )
    };
    let output = render(&mut planner, vec![]);
    let pos = text_center(&output, "Сегодня");
    output.drop_without_applying_deltas();
    for pressed in [true, false] {
        render(
            &mut planner,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        )
        .drop_without_applying_deltas();
    }
    let today = zone.today();
    assert_eq!(
        planner.selection.range(),
        Some(crate::model::DateRange::between(today, today))
    );
}

#[test]
fn dragging_the_menu_corner_resizes_both_axes_and_keeps_the_size_when_reopened() {
    let ctx = egui::Context::default();
    let mut planner = Planner::from_document(Document::default(), &ctx, None);
    let (output, button) = frame(&ctx, &mut planner, vec![]);
    output.drop_without_applying_deltas();
    click(&ctx, &mut planner, button);
    frame(&ctx, &mut planner, vec![])
        .0
        .drop_without_applying_deltas();
    frame(&ctx, &mut planner, vec![egui::Event::Text("moscow".into())])
        .0
        .drop_without_applying_deltas();
    let corner = egui::Id::new("display_timezone_menu_size").with("__resize_corner");
    let original = ctx.read_response(corner).unwrap().rect.center();
    let target = original + egui::vec2(110.0, 70.0);
    let pointer = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    frame(
        &ctx,
        &mut planner,
        vec![egui::Event::PointerMoved(original), pointer(original, true)],
    )
    .0
    .drop_without_applying_deltas();
    for _ in 0..2 {
        frame(&ctx, &mut planner, vec![egui::Event::PointerMoved(target)])
            .0
            .drop_without_applying_deltas();
    }
    frame(&ctx, &mut planner, vec![pointer(target, false)])
        .0
        .drop_without_applying_deltas();
    assert!(egui::Popup::is_any_open(&ctx));
    assert_eq!(planner.timezone_picker.search.query, "moscow");
    let resized = ctx.read_response(corner).unwrap().rect.center();
    assert!(resized.x > original.x + 90.0);
    assert!(resized.y > original.y + 50.0);
    frame(
        &ctx,
        &mut planner,
        vec![egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
    )
    .0
    .drop_without_applying_deltas();
    assert!(!egui::Popup::is_any_open(&ctx));
    click(&ctx, &mut planner, button);
    frame(&ctx, &mut planner, vec![])
        .0
        .drop_without_applying_deltas();
    assert_eq!(ctx.read_response(corner).unwrap().rect.center(), resized);
}
