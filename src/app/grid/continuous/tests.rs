use super::*;
use crate::{
    calendar::Region,
    model::{CalendarViewMode, DateRange, Document, EventSchedule, Frequency, Recurrence},
};

fn planner(ctx: &egui::Context) -> Planner {
    Planner::from_document(
        Document {
            year: Year::try_from(2026).unwrap(),
            view_mode: CalendarViewMode::Continuous,
            ..Document::default()
        },
        ctx,
        None,
    )
}

#[test]
fn changing_display_timezone_reprojects_cached_neighboring_years() {
    use crate::model::{CategoryId, DisplayTimeZone};
    let ctx = egui::Context::default();
    let mut planner = planner(&ctx);
    planner.document.display_timezone = DisplayTimeZone::Utc;
    let file = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:boundary\r\nDTSTART:20261231T233000Z\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
    planner
        .document
        .merge_events(crate::ical::import(file, CategoryId(1)).unwrap());
    let year = |year| Year::try_from(year).unwrap();
    assert_eq!(
        planner
            .continuous
            .year_mut(&planner.document, year(2026))
            .events
            .months[11]
            .days[30]
            .colors()
            .len(),
        1
    );
    assert_eq!(
        planner
            .continuous
            .year_mut(&planner.document, year(2027))
            .events
            .months[0]
            .days[0]
            .colors(),
        [] as [[u8; 3]; 0]
    );
    planner.change_display_timezone(DisplayTimeZone::Named(chrono_tz::Europe::Moscow));
    assert_eq!(
        planner
            .continuous
            .year_mut(&planner.document, year(2026))
            .events
            .months[11]
            .days[30]
            .colors(),
        [] as [[u8; 3]; 0]
    );
    assert_eq!(
        planner
            .continuous
            .year_mut(&planner.document, year(2027))
            .events
            .months[0]
            .days[0]
            .colors()
            .len(),
        1
    );
    assert_eq!(planner.document.year.get(), 2026);
}

fn frame(
    ctx: &egui::Context,
    planner: &mut Planner,
    width: f32,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(
                egui::Pos2::ZERO,
                Vec2::new(width, 800.0),
            )),
            events,
            ..Default::default()
        },
        |ui| planner.year_grid(ui),
    )
}

#[test]
fn six_column_navigation_preserves_the_requested_year() {
    let ctx = egui::Context::default();
    let mut planner = planner(&ctx);
    let width = grid_size(6, Geometry::new(false)).x + 100.0;
    frame(&ctx, &mut planner, width, vec![]).drop_without_applying_deltas();
    assert_eq!(planner.document.year.get(), 2026);
    for delta in [-1, -1, 1, 1, 1] {
        let target = planner.document.year.step(delta).unwrap();
        planner.change_year(target);
        for _ in 0..3 {
            frame(&ctx, &mut planner, width, vec![]).drop_without_applying_deltas();
            assert_eq!(planner.document.year, target);
        }
    }
    for year in [1900, 2100] {
        let target = Year::try_from(year).unwrap();
        planner.change_year(target);
        frame(&ctx, &mut planner, width, vec![]).drop_without_applying_deltas();
        assert_eq!(planner.document.year, target);
    }
    for date in ["2026-01-01", "2026-06-15", "2026-12-31"] {
        let date = date.parse::<NaiveDate>().unwrap();
        planner.go_to_date(date).unwrap();
        frame(&ctx, &mut planner, width, vec![]).drop_without_applying_deltas();
        assert_eq!(planner.document.year.get(), date.year());
        assert_eq!(
            planner.selection.range(),
            Some(DateRange::between(date, date))
        );
    }
}

#[test]
fn tall_viewport_keeps_navigation_stable_at_year_bounds() {
    let ctx = egui::Context::default();
    let mut planner = planner(&ctx);
    for year in [1900, 2026, 2100] {
        let target = Year::try_from(year).unwrap();
        planner.change_year(target);
        for _ in 0..3 {
            ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(
                        egui::Pos2::ZERO,
                        Vec2::new(grid_size(6, Geometry::new(false)).x + 100.0, 1600.0),
                    )),
                    ..Default::default()
                },
                |ui| planner.year_grid(ui),
            )
            .drop_without_applying_deltas();
            assert_eq!(planner.document.year, target);
        }
    }
}

#[test]
fn wheel_scroll_updates_active_year_without_clearing_selection_and_stays_bounded() {
    let ctx = egui::Context::default();
    let mut planner = planner(&ctx);
    let range = DateRange::between("2026-12-30".parse().unwrap(), "2027-01-03".parse().unwrap());
    planner.selection.set_range(range);
    frame(&ctx, &mut planner, 1000.0, vec![]).drop_without_applying_deltas();
    let (stride, before) = planner.continuous.geometry.unwrap();
    for _ in 0..8 {
        frame(
            &ctx,
            &mut planner,
            1000.0,
            vec![
                egui::Event::PointerMoved(egui::pos2(500.0, 400.0)),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    phase: egui::TouchPhase::Move,
                    delta: Vec2::new(0.0, -stride / 2.0),
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        )
        .drop_without_applying_deltas();
    }
    assert!(planner.continuous.geometry.unwrap().1 > before);
    assert!(planner.document.year.get() > 2026);
    assert_eq!(planner.selection.range(), Some(range));
    assert!(planner.continuous.years.len() <= 5);
    for year in [1900, 2100] {
        planner.change_year(Year::try_from(year).unwrap());
        frame(&ctx, &mut planner, 1000.0, vec![]).drop_without_applying_deltas();
        assert_eq!(planner.document.year.get(), year);
        assert!(
            planner
                .continuous
                .years
                .iter()
                .all(|entry| (1900..=2100).contains(&entry.calendar.year.get()))
        );
    }
}

#[test]
fn navigation_resize_mode_switch_and_restore_preserve_the_intended_anchor() {
    let ctx = egui::Context::default();
    let mut planner = planner(&ctx);
    frame(&ctx, &mut planner, 1000.0, vec![]).drop_without_applying_deltas();
    planner.go_to_date("2027-06-15".parse().unwrap()).unwrap();
    frame(&ctx, &mut planner, 1000.0, vec![]).drop_without_applying_deltas();
    let selected = planner.selection.range();
    let (stride, offset) = planner.continuous.geometry.unwrap();
    assert_eq!(planner.document.year.get(), 2027);
    frame(&ctx, &mut planner, 1600.0, vec![]).drop_without_applying_deltas();
    let (new_stride, new_offset) = planner.continuous.geometry.unwrap();
    assert!((offset / stride - new_offset / new_stride).abs() < 0.001);
    assert_eq!(planner.selection.range(), selected);
    planner.change_view_mode(CalendarViewMode::SingleYear);
    planner.change_view_mode(CalendarViewMode::Continuous);
    frame(&ctx, &mut planner, 1000.0, vec![]).drop_without_applying_deltas();
    assert_eq!(planner.selection.range(), selected);
    let (stride, offset) = planner.continuous.geometry.unwrap();
    assert!((offset - year_offset(planner.document.year, stride)).abs() < 0.1);
    let restored = Document {
        year: Year::try_from(2000).unwrap(),
        view_mode: CalendarViewMode::Continuous,
        ..Document::default()
    };
    planner.replace_document(restored, &ctx);
    frame(&ctx, &mut planner, 1000.0, vec![]).drop_without_applying_deltas();
    assert_eq!(planner.document.year.get(), 2000);
}

#[test]
fn cached_years_project_cross_year_events_recurrence_visibility_and_region() {
    let ctx = egui::Context::default();
    let mut planner = planner(&ctx);
    planner.document.add_examples();
    planner.document.events.truncate(2);
    let range = DateRange::between("2026-12-30".parse().unwrap(), "2027-01-03".parse().unwrap());
    planner.document.events[0].schedule = EventSchedule::all_day(range);
    let date = "2026-01-02".parse().unwrap();
    planner.document.events[1].category = planner.document.events[0].category;
    planner.document.events[1].schedule = EventSchedule::all_day(DateRange::between(date, date))
        .with_recurrence(Some(
            Recurrence::new(
                Frequency::Yearly,
                std::num::NonZeroU16::new(1).unwrap(),
                None,
            )
            .unwrap(),
        ))
        .unwrap();
    planner.mark_changed();
    for year in [2026, 2027] {
        planner.change_year(Year::try_from(year).unwrap());
        frame(&ctx, &mut planner, 1000.0, vec![]).drop_without_applying_deltas();
        let entry = planner
            .continuous
            .years
            .iter()
            .find(|entry| entry.calendar.year.get() == year)
            .unwrap();
        let month = if year == 2026 { 11 } else { 0 };
        assert!(!entry.events.months[month].events.is_empty());
        assert_ne!(entry.events.months[0].days[1].colors(), [] as [[u8; 3]; 0]);
    }
    planner.document.groups[0].enabled = false;
    planner.document.region = Region::Tatarstan;
    planner.mark_changed();
    frame(&ctx, &mut planner, 1000.0, vec![]).drop_without_applying_deltas();
    for entry in &planner.continuous.years {
        assert!(entry.calendar.region == Region::Tatarstan);
        assert!(
            entry
                .events
                .months
                .iter()
                .all(|month| month.events.is_empty())
        );
    }
}

#[test]
fn separator_spans_calendar_width_and_labels_straddle_the_line() {
    let ctx = egui::Context::default();
    let rect = Rect::from_min_size(egui::pos2(20.0, 30.0), Vec2::new(720.0, SEPARATOR_HEIGHT));
    let output = ctx.run_ui(egui::RawInput::default(), |ui| {
        year_separator(ui, rect, Year::try_from(2026).unwrap());
    });
    assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::LineSegment { points, .. } if points[0] == egui::pos2(rect.left(), rect.center().y) && points[1] == egui::pos2(rect.right(), rect.center().y))));
    for (year, above) in [("2026", true), ("2027", false)] {
        let text = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.text() == year => Some(text),
                _ => None,
            })
            .unwrap();
        assert!((text.pos.x + text.galley.size().x - rect.right()).abs() < 0.1);
        assert_eq!(text.pos.y < rect.center().y, above);
    }
    output.drop_without_applying_deltas();
}

#[test]
fn narrow_calendar_scrolls_horizontally_without_changing_vertical_position() {
    let ctx = egui::Context::default();
    let mut planner = planner(&ctx);
    let output = frame(&ctx, &mut planner, 300.0, vec![]);
    let month_x = |output: &egui::FullOutput| {
        output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.text() == "Январь" => Some(text.pos.x),
                _ => None,
            })
            .unwrap()
    };
    let before_x = month_x(&output);
    let before_y = planner.continuous.geometry.unwrap().1;
    output.drop_without_applying_deltas();
    for _ in 0..6 {
        frame(
            &ctx,
            &mut planner,
            300.0,
            vec![
                egui::Event::PointerMoved(egui::pos2(150.0, 400.0)),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    phase: egui::TouchPhase::Move,
                    delta: Vec2::new(-80.0, 0.0),
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        )
        .drop_without_applying_deltas();
    }
    let output = frame(&ctx, &mut planner, 300.0, vec![]);
    assert!(month_x(&output) < before_x);
    assert!((planner.continuous.geometry.unwrap().1 - before_y).abs() < 0.1);
    output.drop_without_applying_deltas();
}

#[test]
fn double_click_on_a_neighboring_year_selects_its_date_and_creates_an_event() {
    let ctx = egui::Context::default();
    let mut planner = planner(&ctx);
    planner.change_year(Year::try_from(2027).unwrap());
    let output = frame(&ctx, &mut planner, 1000.0, vec![]);
    let day = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.text() == "2" => {
                Some(text.pos + text.galley.size() / 2.0)
            }
            _ => None,
        })
        .unwrap();
    output.drop_without_applying_deltas();
    for _ in 0..2 {
        for pressed in [true, false] {
            frame(
                &ctx,
                &mut planner,
                1000.0,
                vec![
                    egui::Event::PointerMoved(day),
                    egui::Event::PointerButton {
                        pos: day,
                        pressed,
                        button: egui::PointerButton::Primary,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            )
            .drop_without_applying_deltas();
        }
    }
    let date = "2027-01-02".parse().unwrap();
    assert_eq!(
        planner.selection.range(),
        Some(DateRange::between(date, date))
    );
    assert!(planner.event_draft.is_some());
}

#[test]
fn frequent_wheel_events_accelerate_only_over_the_calendar() {
    let distance = |interval: f64, pointer: egui::Pos2| {
        let ctx = egui::Context::default();
        let mut planner = planner(&ctx);
        let mut render = |time, events| {
            ctx.run_ui(
                egui::RawInput {
                    time: Some(time),
                    screen_rect: Some(Rect::from_min_size(
                        egui::Pos2::ZERO,
                        Vec2::new(1000.0, 800.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| planner.year_grid(ui),
            )
            .drop_without_applying_deltas();
        };
        render(0.0, vec![]);
        for index in 0..5 {
            render(
                f64::from(index).mul_add(interval, 0.1),
                vec![
                    egui::Event::PointerMoved(pointer),
                    egui::Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Point,
                        phase: egui::TouchPhase::Move,
                        delta: Vec2::new(0.0, -40.0),
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
        for index in 1..=60 {
            render(f64::from(index) / 60.0 + interval.mul_add(4.0, 0.1), vec![]);
        }
        let (stride, offset) = planner.continuous.geometry.unwrap();
        offset - year_offset(Year::try_from(2026).unwrap(), stride)
    };
    let pointer = egui::pos2(500.0, 400.0);
    let slow = distance(0.3, pointer);
    let fast = distance(0.02, pointer);
    assert!((slow - 200.0).abs() < 1.0, "Slow distance: {slow}");
    assert!(
        fast > slow * 1.5 && fast <= slow * 3.0,
        "Slow: {slow}, fast: {fast}"
    );
    assert!(distance(0.02, egui::pos2(1500.0, 400.0)).abs() < 0.1);
}

#[test]
fn wheel_motion_continues_on_idle_frames_and_navigation_cancels_its_tail() {
    let ctx = egui::Context::default();
    let mut planner = planner(&ctx);
    let render = |planner: &mut Planner, time, events| {
        ctx.run_ui(
            egui::RawInput {
                time: Some(time),
                screen_rect: Some(Rect::from_min_size(
                    egui::Pos2::ZERO,
                    Vec2::new(1000.0, 800.0),
                )),
                events,
                ..Default::default()
            },
            |ui| planner.year_grid(ui),
        )
        .drop_without_applying_deltas();
    };
    render(&mut planner, 0.0, vec![]);
    let before = planner.continuous.geometry.unwrap().1;
    render(
        &mut planner,
        0.1,
        vec![
            egui::Event::PointerMoved(egui::pos2(500.0, 400.0)),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                phase: egui::TouchPhase::Move,
                delta: Vec2::new(0.0, -120.0),
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    let first = planner.continuous.geometry.unwrap().1;
    assert!(first > before && first < before + 120.0);
    render(&mut planner, 0.12, vec![]);
    assert!(planner.continuous.geometry.unwrap().1 > first);
    let year = Year::try_from(2040).unwrap();
    planner.change_year(year);
    render(&mut planner, 0.14, vec![]);
    let (stride, offset) = planner.continuous.geometry.unwrap();
    assert!((offset - year_offset(year, stride)).abs() < 0.1);
    for index in 1..=30 {
        render(&mut planner, f64::from(index).mul_add(0.02, 0.14), vec![]);
        assert!((planner.continuous.geometry.unwrap().1 - offset).abs() < 0.1);
    }
}

#[test]
fn horizontal_scrollbar_requires_two_column_overflow_at_browser_pixel_scales() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let ctx = egui::Context::default();
        let mut planner = planner(&ctx);
        for width in [
            300.0, 471.0, 472.0, 473.0, 720.0, 721.0, 968.0, 969.0, 1216.0, 1217.0, 1464.0, 1465.0,
        ] {
            for _ in 0..3 {
                let mut input = egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(
                        egui::pos2(0.25, 0.25),
                        Vec2::new(width, 800.0),
                    )),
                    ..Default::default()
                };
                input
                    .viewports
                    .get_mut(&egui::ViewportId::ROOT)
                    .unwrap()
                    .native_pixels_per_point = Some(scale);
                ctx.run_ui(input, |ui| {
                    let id = ui.make_persistent_id(egui::IdSalt::new("continuous_horizontal"));
                    planner.year_grid(ui);
                    let state = egui::scroll_area::State::load(&ctx, id).unwrap();
                    let state = serde_json::to_value(state).unwrap();
                    assert_eq!(
                        state["show_scroll"],
                        serde_json::json!({"x": width < grid_size(2, Geometry::new(false)).x, "y": false}),
                        "width={width}, scale={scale}"
                    );
                })
                .drop_without_applying_deltas();
            }
        }
    }
}
