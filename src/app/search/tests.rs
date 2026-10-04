use super::*;
use crate::{
    model::{EventSchedule, Title},
    test_support::{document, event, range, text_rect},
};
use egui::Modifiers;

fn planner(ctx: &egui::Context) -> Planner {
    let mut document = document();
    let category = document.categories().next().unwrap().id;
    let mut item = event(
        1,
        category,
        EventSchedule::all_day(range("2030-10-12", "2030-10-14")),
    );
    item.title = Title::try_from("Планирование").unwrap();
    item.location = "Офис".into();
    document.events.push(item);
    Planner::from_document(document, ctx, None)
}

fn frame(ctx: &egui::Context, planner: &mut Planner, events: Vec<egui::Event>) -> egui::FullOutput {
    ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1200.0, 800.0),
            )),
            events,
            ..egui::RawInput::default()
        },
        |ui| {
            planner.shortcuts(ui.ctx());
            planner.search_panel(ui, false);
        },
    )
}

#[test]
fn search_controls_share_vertical_centers_in_both_languages_and_themes() {
    use crate::text::Language;

    for language in [Language::Russian, Language::English] {
        for dark in [false, true] {
            let ctx = egui::Context::default();
            let mut planner = planner(&ctx);
            planner.document.language = language;
            planner.document.dark = dark;
            appearance::apply_style(&ctx, dark);
            planner.open_search();
            planner.search.input = "dd".into();
            planner.search.query = SearchQuery::parse("dd");
            planner.search.dates = Dates::Custom;
            planner.search.start = "2024-01-01".into();
            planner.search.end = "2024-12-31".into();
            planner.search.parse_filters();
            let output = ctx.run_ui(egui::RawInput::default(), |ui| {
                ui.set_width(324.0);
                planner.sidebar_tabs(ui);
                planner.search_panel(ui, false);
            });
            let rows = [
                [
                    language.text("Календари", "Calendars"),
                    language.text("Поиск", "Search"),
                ],
                ["dd", "×"],
                [
                    language.text("Свой диапазон", "Custom range"),
                    language.text("Все календари", "All calendars"),
                ],
                [language.text("С", "From"), "2024-01-01"],
                ["2024-01-01", language.text("По", "To")],
                ["2024-01-01", "2024-12-31"],
            ]
            .map(|[left, right]| {
                (
                    left,
                    right,
                    text_rect(&output, left).center().y,
                    text_rect(&output, right).center().y,
                )
            });
            output.drop_without_applying_deltas();
            for (left, right, left_y, right_y) in rows {
                assert!(
                    (left_y - right_y).abs() < 0.6,
                    "Misaligned controls: {left} at {left_y}, {right} at {right_y}"
                );
            }
        }
    }
}

#[test]
fn sidebar_tabs_keep_their_positions_during_hover_press_and_selection_changes() {
    use crate::text::Language;

    for language in [Language::Russian, Language::English] {
        for dark in [false, true] {
            for search_open in [false, true] {
                let ctx = egui::Context::default();
                let mut planner = planner(&ctx);
                planner.document.language = language;
                appearance::apply_style(&ctx, dark);
                planner.search.open = search_open;
                let mut render = |events| {
                    let output = ctx.run_ui(
                        egui::RawInput {
                            events,
                            ..egui::RawInput::default()
                        },
                        |ui| {
                            ui.set_width(324.0);
                            planner.sidebar_tabs(ui);
                        },
                    );
                    let rects = [
                        text_rect(&output, language.text("Календари", "Calendars")),
                        text_rect(&output, language.text("Поиск", "Search")),
                    ];
                    output.drop_without_applying_deltas();
                    rects
                };
                let baseline = render(vec![]);
                for rect in baseline {
                    let pos = rect.center();
                    for _ in 0..3 {
                        assert_eq!(render(vec![egui::Event::PointerMoved(pos)]), baseline);
                    }
                    for pressed in [true, false] {
                        assert_eq!(
                            render(vec![egui::Event::PointerButton {
                                pos,
                                button: egui::PointerButton::Primary,
                                pressed,
                                modifiers: Modifiers::NONE,
                            }]),
                            baseline
                        );
                        assert_eq!(render(vec![]), baseline);
                    }
                    assert_eq!(render(vec![egui::Event::PointerGone]), baseline);
                    assert_eq!(render(vec![]), baseline);
                }
            }
        }
    }
}

#[test]
fn invalid_filters_hide_results_instead_of_reusing_the_previous_range() {
    let ctx = egui::Context::default();
    let mut planner = planner(&ctx);
    planner.search.input = "план".into();
    planner.search.query = SearchQuery::parse("план");
    planner.search.dates = Dates::Custom;
    planner.search.start = "2030-10-01".into();
    planner.search.end = "2030-10-31".into();
    planner.search.parse_filters();
    assert!(planner.search.error.is_none());
    planner.search.start = "2030-02-30".into();
    planner.search.parse_filters();
    let output = frame(&ctx, &mut planner, vec![]);
    text_rect(
        &output,
        "Введите даты ГГГГ-ММ-ДД, начало не позже окончания",
    );
    output.drop_without_applying_deltas();
}

#[test]
fn invalid_time_does_not_partially_apply_a_new_date_range() {
    let mut search = Search {
        dates: Dates::Custom,
        start: "2030-10-01".into(),
        end: "2030-10-31".into(),
        after: " 10:30 ".into(),
        ..Search::default()
    };
    search.parse_filters();
    assert!(search.error.is_none());
    let before = search.filters;
    search.start = "2031-10-01".into();
    search.end = "2031-10-31".into();
    search.after = "99:00".into();
    search.parse_filters();
    assert!(matches!(search.error, Some(FilterError::Time)));
    assert!(search.filters == before);
    search.after = "11:30:15".into();
    search.parse_filters();
    assert!(search.error.is_none());
    assert_eq!(
        search.filters.dates,
        Some(range("2031-10-01", "2031-10-31"))
    );
    assert_eq!(search.filters.after, NaiveTime::from_hms_opt(11, 30, 15));
}
