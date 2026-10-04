use crate::test_support::{document, range};
use crate::text::Language;

use super::*;

#[test]
fn new_events_inherit_the_display_timezone() {
    use crate::model::DisplayTimeZone;
    let ctx = egui::Context::default();
    for (display, expected) in [
        (DisplayTimeZone::Utc, EventTimeZone::Utc),
        (
            DisplayTimeZone::Named(chrono_tz::Asia::Tokyo),
            EventTimeZone::Named(chrono_tz::Asia::Tokyo),
        ),
        (
            DisplayTimeZone::System,
            iana_time_zone::get_timezone()
                .ok()
                .and_then(|name| name.parse().ok())
                .map_or(EventTimeZone::Floating, EventTimeZone::Named),
        ),
    ] {
        let mut planner = Planner::from_document(
            Document {
                display_timezone: display,
                ..document()
            },
            &ctx,
            None,
        );
        planner.new_event();
        let draft = planner.event_draft.as_mut().unwrap();
        assert_eq!(draft.timezone, expected);
        draft.title = "Meeting".into();
        draft.all_day = false;
        assert_eq!(
            draft
                .take_event(EventId(1), &planner.document)
                .unwrap()
                .schedule
                .timezone(),
            expected
        );
    }
}

#[test]
fn converting_an_all_day_event_to_timed_uses_the_display_timezone() {
    let ctx = egui::Context::default();
    let mut document = document();
    document.display_timezone = DisplayTimeZone::Named(chrono_tz::Asia::Tokyo);
    let schedule = EventSchedule::all_day(range("2026-10-05", "2026-10-05"));
    document
        .events
        .push(crate::test_support::event(1, CategoryId(1), schedule));
    let mut planner = Planner::from_document(document, &ctx, None);
    planner.edit_event(EventId(1));
    let draft = planner.event_draft.as_mut().unwrap();
    assert_eq!(draft.timezone, EventTimeZone::Named(chrono_tz::Asia::Tokyo));
    assert_eq!(
        draft
            .take_event(EventId(1), &planner.document)
            .unwrap()
            .schedule,
        schedule
    );
    planner.edit_event(EventId(1));
    let draft = planner.event_draft.as_mut().unwrap();
    draft.all_day = false;
    assert_eq!(
        draft
            .take_event(EventId(1), &planner.document)
            .unwrap()
            .schedule
            .timezone(),
        EventTimeZone::Named(chrono_tz::Asia::Tokyo)
    );
}

#[test]
fn changing_series_timezone_preserves_excluded_wall_dates_and_recurrence_end() {
    let input = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:series\r\nSUMMARY:Meeting\r\nDTSTART;TZID=Europe/Berlin:20261005T090000\r\nDTEND;TZID=Europe/Berlin:20261005T100000\r\nRRULE:FREQ=DAILY;UNTIL=20261008T070000Z\r\nEXDATE;TZID=Europe/Berlin:20261007T090000\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
    let mut document = document();
    document.merge_events(crate::ical::import(input, CategoryId(1)).unwrap());
    for zone in [
        EventTimeZone::Utc,
        EventTimeZone::Named(chrono_tz::Asia::Tokyo),
        EventTimeZone::Floating,
    ] {
        let mut draft = EventDraft::from_event(&document.events[0]);
        draft.timezone = zone;
        let saved = draft.take_event(document.events[0].id, &document).unwrap();
        assert_eq!(saved.schedule.timezone(), zone);
        let excluded = saved.identity.exclusions.iter().next().unwrap();
        assert_eq!(excluded.wall_time(zone).to_string(), "2026-10-07 09:00:00");
        assert!(excluded.matches_schedule_type(saved.schedule));
        assert_eq!(saved.occurrences(document.year.range()).count(), 3);
        let mut output = crate::test_support::document();
        output.events.push(saved);
        assert!(Document::parse(&serde_json::to_string(&output).unwrap()).is_ok());
    }
}

#[test]
fn editing_keeps_source_wall_clocks_and_saving_selects_projected_dates() {
    use crate::model::{DisplayTimeZone, Year};
    let ctx = egui::Context::default();
    let mut document = Document {
        year: Year::try_from(2027).unwrap(),
        display_timezone: DisplayTimeZone::Named(chrono_tz::Europe::Moscow),
        ..Document::default()
    };
    let file = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:edited\r\nSUMMARY:Meeting\r\nDTSTART:20261231T233000Z\r\nDTEND:20270101T003000Z\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
    document.merge_events(crate::ical::import(file, CategoryId(1)).unwrap());
    let mut planner = Planner::from_document(document, &ctx, None);
    let event = &planner.document.events[0];
    let id = event.id;
    let schedule = event.schedule;
    let mut draft = EventDraft::from_event(event);
    assert_eq!(draft.start, "2026-12-31");
    assert_eq!(draft.start_time, "23:30");
    assert_eq!(draft.timezone, EventTimeZone::Utc);
    let saved = draft.take_event(id, &planner.document).unwrap();
    planner.save_event(saved);
    let date = "2027-01-01".parse().unwrap();
    assert_eq!(
        planner.selection.range(),
        Some(DateRange::between(date, date))
    );
    assert_eq!(planner.document.events[0].schedule, schedule);
}

#[test]
fn editor_parses_dates_title_and_range_together() {
    let range = DateRange::between("2026-06-01".parse().unwrap(), "2026-06-14".parse().unwrap());
    let mut draft = EventDraft::new(range, CategoryId(7));
    draft.title = " Отпуск ".into();
    let event = draft.take_event(EventId(1), &Document::default()).unwrap();
    assert_eq!(event.title.get(), "Отпуск");
    assert_eq!(event.schedule.occupied_dates().days(), 14);
}

#[test]
fn invalid_dates_or_title_report_the_expected_error_without_consuming_fields() {
    let document = document();
    for (title, start, end, error) in [
        (" ", "2026-06-01", "2026-06-14", InputError::EmptyTitle),
        (
            "Vacation",
            "2026-02-30",
            "2026-06-14",
            InputError::StartDate,
        ),
        ("Vacation", "2026-06-01", "2026-02-30", InputError::EndDate),
        (
            "Vacation",
            "2026-06-01",
            "2026-05-31",
            InputError::ReversedRange,
        ),
        ("Vacation", "1899-12-31", "2026-06-14", InputError::Year),
        ("Vacation", "2026-06-01", "2101-01-01", InputError::Year),
    ] {
        let mut draft = EventDraft::new(range("2026-06-01", "2026-06-14"), CategoryId(7));
        draft.title = title.into();
        draft.start = start.into();
        draft.end = end.into();
        draft.notes = "Keep notes".into();
        draft.location = "Office".into();
        let uid = draft.identity.uid.clone();
        assert!(
            matches!(draft.take_event(EventId(1), &document), Err(actual) if actual == error),
            "{title:?}, {start}..{end}: {error}"
        );
        assert_eq!(draft.title, title);
        assert_eq!(draft.start, start);
        assert_eq!(draft.end, end);
        assert_eq!(draft.notes, "Keep notes");
        assert_eq!(draft.location, "Office");
        assert_eq!(draft.identity.uid, uid);
    }
}

#[test]
fn editor_rejects_a_removed_calendar_without_losing_the_draft() {
    let mut document = Document::default();
    let range = document.year.range();
    let mut draft = EventDraft::new(range, CategoryId(7));
    draft.title = "Vacation".into();
    document.remove_category(CategoryId(7));
    assert!(matches!(
        draft.take_event(EventId(1), &document),
        Err(InputError::UnknownCategory)
    ));
    draft.category = CategoryId(1);
    assert!(draft.take_event(EventId(1), &document).is_ok());
    assert_eq!(draft.title, "Vacation");
}

#[test]
fn editor_preserves_notes_on_error_and_moves_them_on_success() {
    let document = Document::default();
    let mut draft = EventDraft::new(document.year.range(), CategoryId(1));
    draft.notes = "Important details".into();
    assert!(draft.take_event(EventId(1), &document).is_err());
    assert_eq!(draft.notes, "Important details");
    draft.title = "Project".into();
    draft.start = format!("  {}  ", document.year.range().start());
    draft.end = format!("  {}  ", document.year.range().end());
    let event = draft.take_event(EventId(1), &document).unwrap();
    assert_eq!(event.schedule.occupied_dates(), document.year.range());
    assert_eq!(event.notes, "Important details");
    assert_eq!(draft.notes, "");
}

#[test]
fn timed_event_details_survive_persistence_and_editing() {
    let mut document = Document::default();
    let day = "2026-10-02".parse().unwrap();
    let mut draft = EventDraft::new(DateRange::between(day, day), CategoryId(1));
    draft.title = "Meeting".into();
    draft.all_day = false;
    draft.start_time = "13:00:01.125".into();
    draft.end_time = "13:30".into();
    draft.location = "Office".into();
    draft.link = "https://example.com/call".into();
    draft.notes = "Agenda\nDetails".into();
    draft.status = Some(EventStatus::Confirmed);
    draft.availability = Availability::Free;
    document
        .events
        .push(draft.take_event(EventId(1), &document).unwrap());
    let restored = Document::parse(&serde_json::to_string(&document).unwrap()).unwrap();
    let mut editor = EventDraft::from_event(&restored.events[0]);
    assert_eq!(editor.start_time, "13:00:01.125");
    let event = editor.take_event(EventId(1), &restored).unwrap();
    assert_eq!(
        serde_json::to_value(event).unwrap(),
        serde_json::to_value(&restored.events[0]).unwrap()
    );
}

#[test]
fn invalid_time_or_link_preserves_every_text_field() {
    let document = Document::default();
    let day = "2026-10-02".parse().unwrap();
    let mut draft = EventDraft::new(DateRange::between(day, day), CategoryId(1));
    draft.title = "Meeting".into();
    draft.notes = "Details".into();
    draft.location = "Office".into();
    draft.all_day = false;
    for (start, end, link, error) in [
        ("25:00", "13:30", "", InputError::StartTime),
        ("13:00", "bad", "", InputError::EndTime),
        ("13:30", "13:00", "", InputError::ReversedTime),
        ("13:00", "13:30", "example.com", InputError::EventLink),
    ] {
        draft.start_time = start.into();
        draft.end_time = end.into();
        draft.link = link.into();
        assert!(matches!(draft.take_event(EventId(1), &document), Err(actual) if actual == error));
        assert_eq!(draft.title, "Meeting");
        assert_eq!(draft.notes, "Details");
        assert_eq!(draft.location, "Office");
        assert_eq!(draft.link.as_str(), link);
    }
    draft.all_day = true;
    draft.link.clear();
    let event = draft.take_event(EventId(1), &document).unwrap();
    assert_eq!(event.schedule.times(), None);
}

fn dialog_frame(
    ctx: &egui::Context,
    planner: &mut Planner,
    screen: egui::Rect,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(screen),
            events,
            ..Default::default()
        },
        |ui| planner.event_dialog(ui.ctx()),
    )
}

fn click_text(
    ctx: &egui::Context,
    planner: &mut Planner,
    screen: egui::Rect,
    output: egui::FullOutput,
    label: &str,
) {
    click_text_with_discard(ctx, planner, screen, output, label, false);
}

fn click_text_with_discard(
    ctx: &egui::Context,
    planner: &mut Planner,
    screen: egui::Rect,
    output: egui::FullOutput,
    label: &str,
    discard_first_pass: bool,
) {
    let pos = output.shapes.iter().rev().find_map(|shape| {
        if let egui::Shape::Text(text) = &shape.shape
            && text.galley.text() == label
        {
            let pos = text.pos + text.galley.size() / 2.0;
            shape.clip_rect.contains(pos).then_some(pos)
        } else {
            None
        }
    });
    output.drop_without_applying_deltas();
    let pos = pos.unwrap_or_else(|| panic!("Button text is visible: {label}"));
    let events = vec![
        egui::Event::PointerMoved(pos),
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        },
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        },
    ];
    let initial_event_count = planner.document.events.len();
    let initial_editor_state = planner.event_draft.as_ref().map(|draft| draft.state);
    let mut passes = 0;
    ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(screen),
            events,
            ..Default::default()
        },
        |ui| {
            let discarded = discard_first_pass && passes == 0;
            if discarded {
                ui.ctx()
                    .request_discard("Test repeated event editor rendering");
            }
            planner.event_dialog(ui.ctx());
            if discarded {
                assert!(planner.event_draft.is_some());
                assert_eq!(planner.document.events.len(), initial_event_count);
                assert_eq!(
                    planner.event_draft.as_ref().map(|draft| draft.state),
                    initial_editor_state
                );
            }
            passes += 1;
        },
    )
    .drop_without_applying_deltas();
    if discard_first_pass {
        assert!(passes >= 2);
    }
}

#[test]
fn repeated_rendering_applies_reminders_and_save_once() {
    let ctx = egui::Context::default();
    let document = Document {
        language: Language::English,
        ..Document::default()
    };
    let mut planner = Planner::from_document(document, &ctx, None);
    planner.new_event();
    let draft = planner.event_draft.as_mut().unwrap();
    draft.title = "Meeting".into();
    draft.notes = "Keep the description".into();
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1000.0, 900.0));
    settle_dialog(&ctx, &mut planner, screen);
    resize_dialog(&ctx, &mut planner, screen, egui::vec2(0.0, 240.0));
    for label in ["+ Reminder", "+ Reminder", "×", "Save"] {
        let output = dialog_frame(&ctx, &mut planner, screen, Vec::new());
        click_text_with_discard(&ctx, &mut planner, screen, output, label, true);
        settle_dialog(&ctx, &mut planner, screen);
    }
    assert!(planner.event_draft.is_none());
    assert_eq!(planner.document.events.len(), 1);
    let event = &planner.document.events[0];
    assert_eq!(event.notes, "Keep the description");
    assert_eq!(event.details.reminders.len(), 1);
    assert_eq!(event.details.reminders[0].minutes(), 15);
}

fn settle_dialog(ctx: &egui::Context, planner: &mut Planner, screen: egui::Rect) {
    for _ in 0..3 {
        dialog_frame(ctx, planner, screen, Vec::new()).drop_without_applying_deltas();
    }
}

fn resize_dialog(
    ctx: &egui::Context,
    planner: &mut Planner,
    screen: egui::Rect,
    delta: egui::Vec2,
) {
    let window = ctx
        .memory(|memory| memory.area_rect(Dialog::Event.id()))
        .unwrap();
    let start = window.right_bottom() - egui::vec2(6.0, 6.0);
    let end = start + delta;
    for events in [
        vec![egui::Event::PointerMoved(start)],
        vec![egui::Event::PointerButton {
            pos: start,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        }],
        vec![egui::Event::PointerMoved(end)],
        vec![egui::Event::PointerButton {
            pos: end,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }],
    ] {
        dialog_frame(ctx, planner, screen, events).drop_without_applying_deltas();
    }
    settle_dialog(ctx, planner, screen);
}

#[test]
fn resizing_stretches_text_fields_and_description_but_keeps_dates_fixed() {
    let ctx = egui::Context::default();
    let mut document = Document::default();
    let name = "Calendar ".repeat(20);
    document.groups[0].categories[0].name = crate::text::Name::new(&name, &name).unwrap();
    let mut planner = Planner::from_document(document, &ctx, None);
    planner.new_event();
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600.0, 1100.0));
    settle_dialog(&ctx, &mut planner, screen);
    let window = ctx
        .memory(|memory| memory.area_rect(Dialog::Event.id()))
        .unwrap();
    assert!(
        window.width() < 600.0,
        "Long calendar names must fit the selected field"
    );
    resize_dialog(&ctx, &mut planner, screen, egui::vec2(0.0, 320.0));
    let rect = |id| ctx.read_response(Dialog::Event.id().with(id)).unwrap().rect;
    let before = [
        rect("title"),
        rect("location"),
        rect("link"),
        rect("description"),
    ];
    let date_id = Dialog::Event.id().with(("date", "С"));
    let date_before = ctx.read_response(date_id).unwrap().rect;
    resize_dialog(&ctx, &mut planner, screen, egui::vec2(240.0, 180.0));
    let after = [
        rect("title"),
        rect("location"),
        rect("link"),
        rect("description"),
    ];
    for (before, after) in before.into_iter().zip(after) {
        assert!(
            after.width() > before.width() + 200.0,
            "{before:?} -> {after:?}"
        );
    }
    assert!(
        after[3].height() > before[3].height() + 140.0,
        "{:?} -> {:?}",
        before[3],
        after[3]
    );
    assert!((ctx.read_response(date_id).unwrap().rect.width() - date_before.width()).abs() < 0.5);
    resize_dialog(&ctx, &mut planner, screen, egui::vec2(-1000.0, -1000.0));
    let window = ctx
        .memory(|memory| memory.area_rect(Dialog::Event.id()))
        .unwrap();
    assert!(
        window.width() >= 300.0 && window.height() >= 480.0,
        "{window:?}"
    );
}

#[test]
fn calendar_colors_appear_in_selected_value_and_dropdown() {
    let ctx = egui::Context::default();
    let mut document = Document::default();
    let color = [17, 193, 104];
    document.groups[0].categories[0].color = color;
    let name = document.groups[0].categories[0]
        .name
        .get(document.language)
        .to_owned();
    let mut planner = Planner::from_document(document, &ctx, None);
    ctx.global_style_mut(|style| style.animation_time = 0.0);
    planner.new_event();
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1000.0, 900.0));
    settle_dialog(&ctx, &mut planner, screen);
    let output = dialog_frame(&ctx, &mut planner, screen, Vec::new());
    let circle_color = egui::Color32::from_rgb(color[0], color[1], color[2]);
    assert!(!output.shapes.iter().any(
        |shape| matches!(&shape.shape, egui::Shape::Circle(circle) if circle.fill == circle_color)
    ));
    let selected_name = format!("● {name}");
    assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.text() == selected_name && text.galley.job.sections.iter().any(|section| section.format.color == circle_color))));
    click_text(&ctx, &mut planner, screen, output, &selected_name);
    assert!(egui::Popup::is_any_open(&ctx));
    settle_dialog(&ctx, &mut planner, screen);
    let output = dialog_frame(&ctx, &mut planner, screen, Vec::new());
    let has_group_color = output.shapes.iter().any(
        |shape| matches!(&shape.shape, egui::Shape::Circle(circle) if circle.fill == circle_color),
    );
    let has_calendar_color = output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.text() == "●" && text.galley.job.sections.iter().any(|section| section.format.color == circle_color)));
    output.drop_without_applying_deltas();
    assert!(has_group_color);
    assert!(has_calendar_color);
}

#[test]
fn editing_an_import_preserves_identity_zone_recurrence_and_exclusions() {
    let input = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:meeting@example.org\r\nSUMMARY:Meeting\r\nDTSTART;TZID=Europe/Berlin:20261005T090000\r\nDTEND;TZID=Europe/Berlin:20261005T100000\r\nRRULE:FREQ=WEEKLY;BYDAY=MO,WE;COUNT=6;WKST=SU\r\nEXDATE;TZID=Europe/Berlin:20261007T090000\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
    let mut document = Document::default();
    document.merge_events(crate::ical::import(input, CategoryId(1)).unwrap());
    let event = &document.events[0];
    let expected = serde_json::to_value(event).unwrap();
    let mut draft = EventDraft::from_event(event);
    let saved = draft.take_event(event.id, &document).unwrap();
    assert_eq!(serde_json::to_value(saved).unwrap(), expected);
}

#[test]
fn changing_series_time_or_all_day_keeps_excluded_days_and_valid_persistence() {
    let input = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:series\r\nSUMMARY:Meeting\r\nDTSTART;TZID=Europe/Berlin:20261005T090000\r\nDTEND;TZID=Europe/Berlin:20261005T100000\r\nRRULE:FREQ=DAILY;UNTIL=20261008T070000Z\r\nEXDATE;TZID=Europe/Berlin:20261007T090000\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
    let mut document = Document::default();
    document.merge_events(crate::ical::import(input, CategoryId(1)).unwrap());
    for all_day in [false, true] {
        let mut draft = EventDraft::from_event(&document.events[0]);
        draft.start_time = "08:00".into();
        draft.all_day = all_day;
        let saved = draft.take_event(document.events[0].id, &document).unwrap();
        assert_eq!(
            saved
                .occurrences(crate::model::Year::try_from(2026).unwrap().range())
                .count(),
            3
        );
        let mut output = Document::default();
        output.events.push(saved);
        assert!(Document::parse(&serde_json::to_string(&output).unwrap()).is_ok());
    }
}
