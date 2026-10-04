use super::*;

#[test]
fn event_highlight_tracks_its_day_during_continuous_scroll_at_fractional_scale() {
    let mut document = document();
    document.view_mode = CalendarViewMode::Continuous;
    document.dark = true;
    document.events.clear();
    let category = document.categories().next().unwrap();
    let color = egui::Color32::from_rgb(category.color[0], category.color[1], category.color[2])
        .gamma_multiply(0.30);
    document.events.push(event(
        1,
        category.id,
        EventSchedule::all_day(range("2026-01-11", "2026-01-11")),
    ));
    let mut harness = harness(document, egui::vec2(1440.0, 960.0));
    harness.set_pixels_per_point(1.1);
    harness.run_steps(3);
    let before = state(&harness).calendar.unwrap().offset;
    let initial_center = harness
        .get_by_role_and_label(Role::Button, "2026-01-11")
        .rect()
        .center();
    for _ in 0..20 {
        let pointer = state(&harness).calendar.unwrap().rect.center();
        harness.input_mut().events.extend([
            egui::Event::PointerMoved(pointer),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                phase: egui::TouchPhase::Move,
                delta: egui::vec2(0.0, -1.5),
                modifiers: Modifiers::NONE,
            },
        ]);
        harness.step();
        let day_center = harness
            .get_by_role_and_label(Role::Button, "2026-01-11")
            .rect()
            .center();
        let highlight_center = harness
            .output()
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Circle(circle) if circle.fill == color => Some(circle.center),
                _ => None,
            })
            .expect("Single-day event highlight is missing");
        assert!(highlight_center.distance(day_center) < 0.01);
    }
    assert!(state(&harness).calendar.unwrap().offset > before);
    let final_center = harness
        .get_by_role_and_label(Role::Button, "2026-01-11")
        .rect()
        .center();
    assert!(final_center.y < initial_center.y);
}
