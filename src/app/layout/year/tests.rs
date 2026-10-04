use crate::model::{Document, Year};
use crate::test_support::text_rect;

use super::*;

fn picker_frame(
    ctx: &egui::Context,
    planner: &mut Planner,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(600.0, 200.0),
            )),
            events,
            ..Default::default()
        },
        |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                planner.year_picker(
                    ui,
                    egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(600.0, 60.0)),
                );
                let _ = ui.button("Outside");
            });
        },
    )
}

fn year_picker_centers(year: i32) -> (egui::Pos2, [Option<egui::Pos2>; 2]) {
    let ctx = egui::Context::default();
    let document = Document {
        year: Year::try_from(year).unwrap(),
        ..crate::test_support::document()
    };
    let mut planner = Planner::from_document(document, &ctx, None);
    let output = picker_frame(&ctx, &mut planner, vec![]);
    let arrows = ["<", ">"].map(|label| {
        output.shapes.iter().find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.text() == label => {
                Some(text.pos + text.galley.size() / 2.0)
            }
            _ => None,
        })
    });
    let centers = (text_rect(&output, &year.to_string()).center(), arrows);
    output.drop_without_applying_deltas();
    centers
}

#[test]
fn year_change_does_not_shift_the_arrow_buttons() {
    let (_, [left, right]) = year_picker_centers(2000);
    for year in [1900, 2026, 2100] {
        let (_, [left_after, right_after]) = year_picker_centers(year);
        assert_eq!(
            left_after,
            if year == 1900 { None } else { left },
            "left arrow in {year}"
        );
        assert_eq!(
            right_after,
            if year == 2100 { None } else { right },
            "right arrow in {year}"
        );
    }
}

#[test]
fn year_label_is_centered_and_stable_across_years() {
    let center = year_picker_centers(2000).0.x;
    assert!(
        (center - 300.0).abs() < 1.0,
        "Label is not centered: {center}"
    );
    for year in [1900, 2026, 2100] {
        let after = year_picker_centers(year).0.x;
        assert!(
            (center - after).abs() < 0.5,
            "Label drifts in {year}: {center} vs {after}"
        );
    }
}
