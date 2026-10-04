use egui::{Color32, Vec2};

use super::*;

#[test]
fn date_navigation_checks_boundaries_before_changing_state() {
    let ctx = egui::Context::default();
    let mut planner = Planner::from_document(Document::default(), &ctx, None);
    let date = NaiveDate::from_ymd_opt(2024, 2, 29).unwrap();
    planner.go_to_date(date).unwrap();
    assert_eq!(planner.document.year.get(), 2024);
    assert_eq!(
        planner.selection.range().unwrap(),
        DateRange::between(date, date)
    );
    let selection = planner.selection;
    for year in [1899, 2101] {
        assert!(matches!(
            planner.go_to_date(NaiveDate::from_ymd_opt(year, 1, 1).unwrap()),
            Err(InputError::Year)
        ));
        assert_eq!(planner.document.year.get(), 2024);
        assert_eq!(planner.selection, selection);
    }
}

fn input(size: Vec2, mut events: Vec<egui::Event>, modifiers: egui::Modifiers) -> egui::RawInput {
    events.insert(0, egui::Event::ModifiersChanged(modifiers));
    egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
        events,
        ..Default::default()
    }
}

#[test]
fn dropdown_rows_keep_their_rects_during_interaction() {
    for dark in [false, true] {
        for selected in [false, true] {
            let ctx = egui::Context::default();
            appearance::apply_style(&ctx, dark);
            let mut rows = [egui::Rect::NOTHING; 3];
            let mut render = |events, focus| {
                ctx.run_ui(
                    input(Vec2::new(400.0, 300.0), events, egui::Modifiers::NONE),
                    |ui| {
                        appearance::dropdown_style(ui.style_mut());
                        for (index, rect) in rows.iter_mut().enumerate() {
                            let response = ui.selectable_label(selected, "Dropdown row");
                            *rect = response.rect;
                            if index == 0 && focus {
                                response.request_focus();
                            }
                        }
                    },
                )
                .drop_without_applying_deltas();
                rows
            };
            let baseline = render(Vec::new(), false);
            assert_eq!(render(Vec::new(), false), baseline);
            let pointer = baseline[0].center();
            for _ in 0..3 {
                assert_eq!(
                    render(vec![egui::Event::PointerMoved(pointer)], false),
                    baseline
                );
            }
            assert_eq!(
                render(
                    vec![egui::Event::PointerButton {
                        pos: pointer,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::NONE,
                    }],
                    false,
                ),
                baseline
            );
            assert_eq!(render(Vec::new(), false), baseline);
            assert_eq!(
                render(
                    vec![
                        egui::Event::PointerButton {
                            pos: pointer,
                            button: egui::PointerButton::Primary,
                            pressed: false,
                            modifiers: egui::Modifiers::NONE,
                        },
                        egui::Event::PointerGone,
                    ],
                    true,
                ),
                baseline
            );
            assert_eq!(render(Vec::new(), true), baseline);
        }
    }
}

#[test]
fn render_all_layouts_themes_and_languages() {
    for size in [
        Vec2::new(760.0, 600.0),
        Vec2::new(1120.0, 800.0),
        Vec2::new(1536.0, 1024.0),
        Vec2::new(1600.0, 1024.0),
        Vec2::new(1920.0, 1080.0),
    ] {
        for (language, view_mode) in
            [Language::Russian, Language::English]
                .into_iter()
                .flat_map(|language| {
                    [CalendarViewMode::SingleYear, CalendarViewMode::Continuous]
                        .map(|mode| (language, mode))
                })
        {
            for dark in [false, true] {
                let ctx = egui::Context::default();
                let mut document = Document {
                    dark,
                    language,
                    view_mode,
                    year: Year::try_from(2026).unwrap(),
                    ..Document::default()
                };
                document.add_examples();
                let mut planner = Planner::from_document(document, &ctx, None);
                planner.frame_cpu_seconds = Some(0.001);
                for _ in 0..2 {
                    let output = ctx.run_ui(input(size, Vec::new(), egui::Modifiers::NONE), |ui| {
                        planner.render(ui);
                    });
                    assert_ne!(output.shapes.len(), 0);
                    let indicator = crate::test_support::text_rect(
                        &output,
                        language.text("Задержка кадра: 1.0 мс", "Frame delay: 1.0 ms"),
                    );
                    let footer =
                        crate::test_support::text_rect(&output, language.text("Сегодня", "Today"));
                    assert!(indicator.bottom() < footer.top());
                    assert!(indicator.right() > size.x - 50.0);
                    output.drop_without_applying_deltas();
                }
            }
        }
    }
}

#[test]
fn grid_updates_after_visibility_color_and_document_changes() {
    let ctx = egui::Context::default();
    let mut document = Document {
        year: Year::try_from(2026).unwrap(),
        ..Document::default()
    };
    document.add_examples();
    document.events.truncate(1);
    let mut planner = Planner::from_document(document, &ctx, None);
    let render_markers = |planner: &mut Planner| {
        let output = ctx.run_ui(
            input(Vec2::new(2000.0, 1400.0), Vec::new(), egui::Modifiers::NONE),
            |ui| planner.render(ui),
        );
        let markers: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Circle(circle) if (circle.radius - 1.5).abs() < f32::EPSILON => {
                    Some(circle.fill)
                }
                _ => None,
            })
            .collect();
        output.drop_without_applying_deltas();
        markers
    };
    assert_eq!(render_markers(&mut planner).len(), 1);
    planner.document.groups[0].enabled = false;
    planner.mark_changed();
    assert_eq!(render_markers(&mut planner), []);
    planner.document.groups[0].enabled = true;
    planner.document.groups[0].categories[1].color = [1, 2, 3];
    planner.mark_changed();
    assert_eq!(render_markers(&mut planner), [Color32::from_rgb(1, 2, 3)]);
    planner.change_year(Year::try_from(2027).unwrap());
    assert_eq!(render_markers(&mut planner), []);
    planner.new_event();
    planner.tree_draft = Some(TreeDraft::new(TreeTarget::Group(0), "Work"));
    planner
        .selection
        .start_drag(planner.document.year.range().start());
    planner.replace_document(Document::default(), &ctx);
    assert!(planner.event_draft.is_none());
    assert!(planner.tree_draft.is_none());
    assert_eq!(planner.selection, Selection::Empty);
    assert_eq!(render_markers(&mut planner), []);
}

#[test]
fn date_click_shift_click_and_drag_select_inclusive_range() {
    let ctx = egui::Context::default();
    let document = Document {
        year: Year::try_from(2026).unwrap(),
        ..Document::default()
    };
    let mut planner = Planner::from_document(document, &ctx, None);
    let size = Vec2::new(1200.0, 900.0);
    let mut pass = |events, modifiers| {
        let output = ctx.run_ui(input(size, events, modifiers), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                planner.year_grid(ui);
            });
        });
        let mut positions = [egui::Pos2::ZERO; 5];
        for shape in &output.shapes {
            if let egui::Shape::Text(text) = &shape.shape {
                if text.galley.text() == "Февраль" {
                    break;
                }
                if let Ok(day) = text.galley.text().parse::<usize>()
                    && let Some(position) = day
                        .checked_sub(1)
                        .and_then(|index| positions.get_mut(index))
                {
                    *position = text.pos + text.galley.size() / 2.0;
                }
            }
        }
        output.drop_without_applying_deltas();
        positions
    };
    pass(Vec::new(), egui::Modifiers::NONE);
    let positions = pass(Vec::new(), egui::Modifiers::NONE);
    let start = positions[0];
    let end = positions[2];
    let pointer = |pos, pressed, modifiers| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers,
    };
    pass(
        vec![
            egui::Event::PointerMoved(start),
            pointer(start, true, egui::Modifiers::NONE),
        ],
        egui::Modifiers::NONE,
    );
    pass(
        vec![pointer(start, false, egui::Modifiers::NONE)],
        egui::Modifiers::NONE,
    );
    pass(
        vec![
            egui::Event::PointerMoved(end),
            pointer(end, true, egui::Modifiers::SHIFT),
        ],
        egui::Modifiers::SHIFT,
    );
    pass(
        vec![pointer(end, false, egui::Modifiers::SHIFT)],
        egui::Modifiers::SHIFT,
    );
    assert_eq!(planner.selection.range().unwrap().days(), 3);
    assert_eq!(
        planner.selection.range().unwrap().start().to_string(),
        "2026-01-01"
    );
    planner.selection.clear();
    let drag_end = positions[4];
    let mut pass = |events| {
        ctx.run_ui(input(size, events, egui::Modifiers::NONE), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                planner.year_grid(ui);
            });
        })
        .drop_without_applying_deltas();
    };
    pass(vec![
        egui::Event::PointerMoved(start),
        pointer(start, true, egui::Modifiers::NONE),
    ]);
    pass(vec![egui::Event::PointerMoved(drag_end)]);
    pass(vec![
        egui::Event::PointerMoved(end),
        pointer(end, false, egui::Modifiers::NONE),
    ]);
    pass(vec![egui::Event::PointerMoved(drag_end)]);
    assert_eq!(planner.selection.range().unwrap().days(), 3);
    assert!(!planner.selection.is_dragging());
}

#[test]
fn resizing_keeps_days_and_month_spacing_unchanged() {
    let ctx = egui::Context::default();
    let document = Document {
        year: Year::try_from(2026).unwrap(),
        ..Document::default()
    };
    let mut planner = Planner::from_document(document, &ctx, None);
    let mut reference = None;
    for size in [
        Vec2::new(1536.0, 1024.0),
        Vec2::new(1000.0, 760.0),
        Vec2::new(760.0, 600.0),
        Vec2::new(1280.0, 1000.0),
        Vec2::new(1600.0, 1000.0),
        Vec2::new(2000.0, 1400.0),
    ] {
        let mut positions = Vec::new();
        for _ in 0..2 {
            let output = ctx.run_ui(input(size, Vec::new(), egui::Modifiers::NONE), |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    planner.year_grid(ui);
                });
            });
            positions.clear();
            let mut origin = None;
            for shape in &output.shapes {
                if let egui::Shape::Text(text) = &shape.shape {
                    match text.galley.text() {
                        "Январь" => origin = Some(text.pos),
                        "Февраль" => {
                            if let Some(origin) = origin {
                                positions.push(("Февраль".to_owned(), text.pos - origin));
                            }
                            break;
                        }
                        label if label.parse::<u8>().is_ok() => {
                            if let Some(origin) = origin {
                                positions.push((label.to_owned(), text.pos - origin));
                            }
                        }
                        _ => {}
                    }
                }
            }
            output.drop_without_applying_deltas();
        }
        assert_eq!(positions.len(), 32);
        if let Some(reference) = &reference {
            assert_eq!(&positions, reference);
        } else {
            reference = Some(positions);
        }
    }
}

#[test]
fn selection_caps_follow_endpoint_circles() {
    let ctx = egui::Context::default();
    let document = Document {
        year: Year::try_from(2025).unwrap(),
        ..Document::default()
    };
    let mut planner = Planner::from_document(document, &ctx, None);
    planner.selection.set_range(DateRange::between(
        "2025-05-13".parse().unwrap(),
        "2025-05-17".parse().unwrap(),
    ));
    let output = ctx.run_ui(
        input(Vec2::new(1200.0, 900.0), Vec::new(), egui::Modifiers::NONE),
        |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                planner.year_grid(ui);
            });
        },
    );
    let mut circles = Vec::new();
    let mut bar = None;
    for shape in &output.shapes {
        match &shape.shape {
            egui::Shape::Circle(circle) if circle.fill == BLUE => {
                circles.push(circle.visual_bounding_rect());
            }
            egui::Shape::Rect(rect) if rect.stroke.color == BLUE => bar = Some(rect.rect),
            _ => {}
        }
    }
    assert_eq!(circles.len(), 2);
    assert_eq!(bar.unwrap(), circles[0].union(circles[1]));
    output.drop_without_applying_deltas();
}
