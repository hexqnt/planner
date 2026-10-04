use egui::{Color32, Response, Ui, Widget};

use crate::{app::ui_config::sidebar::COLOR_BUTTON_SIZE, text::Language};

const DOT_RADIUS: f32 = 6.0;
const PRESETS: [([u8; 3], &str); 8] = [
    ([39, 133, 245], "#2785F5"),
    ([17, 193, 104], "#11C168"),
    ([240, 180, 40], "#F0B428"),
    ([239, 93, 93], "#EF5D5D"),
    ([163, 107, 240], "#A36BF0"),
    ([40, 190, 200], "#28BEC8"),
    ([238, 130, 180], "#EE82B4"),
    ([130, 140, 155], "#828C9B"),
];

pub(in crate::app) struct CalendarColorInput<'a> {
    rgb: &'a mut [u8; 3],
    language: Language,
}

impl<'a> CalendarColorInput<'a> {
    pub(in crate::app) const fn new(rgb: &'a mut [u8; 3], language: Language) -> Self {
        Self { rgb, language }
    }
}

impl Widget for CalendarColorInput<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let before = *self.rgb;
        let [r, g, b] = before;
        let mut color = Color32::from_rgb(r, g, b);
        let (rect, response) =
            ui.allocate_exact_size(egui::Vec2::splat(COLOR_BUTTON_SIZE), egui::Sense::click());
        ui.painter().circle_filled(rect.center(), DOT_RADIUS, color);
        let mut response = super::labelled_action(
            response,
            self.language.text("Цвет календаря", "Calendar color"),
        );
        egui::Popup::menu(&response).show(|ui| {
            ui.horizontal(|ui| {
                for ([r, g, b], label) in PRESETS {
                    let preset = Color32::from_rgb(r, g, b);
                    let button = ui.add(
                        egui::Button::new(" ")
                            .fill(preset)
                            .min_size(egui::Vec2::splat(COLOR_BUTTON_SIZE)),
                    );
                    if super::labelled_action(button, label).clicked() {
                        color = preset;
                    }
                }
            });
            egui::color_picker::color_picker_color32(
                ui,
                &mut color,
                egui::color_picker::Alpha::Opaque,
            );
        });
        *self.rgb = [color.r(), color.g(), color.b()];
        if before != *self.rgb {
            response.mark_changed();
        }
        response
    }
}
