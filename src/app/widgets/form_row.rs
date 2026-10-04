use egui::{Align, Image, ImageSource, InnerResponse, Layout, TextStyle, Ui, Vec2, vec2};

use crate::app::ui_config::dialog;

pub(in crate::app) struct FormRow<'a> {
    icon: ImageSource<'a>,
    label: &'a str,
}

impl<'a> FormRow<'a> {
    pub(in crate::app) const fn new(icon: ImageSource<'a>, label: &'a str) -> Self {
        Self { icon, label }
    }

    pub(in crate::app) fn show<R>(
        self,
        ui: &mut Ui,
        body: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        let response = ui.horizontal_top(|ui| {
            ui.spacing_mut().button_padding.y = ui
                .spacing()
                .button_padding
                .y
                .max(dialog::FIELD_VERTICAL_PADDING);
            let height = 2.0_f32
                .mul_add(
                    ui.spacing().button_padding.y,
                    ui.text_style_height(&TextStyle::Button),
                )
                .max(ui.spacing().interact_size.y)
                .max(dialog::FIELD_ICON_SIZE);
            ui.spacing_mut().interact_size.y = height;
            ui.add_sized(
                vec2(dialog::FIELD_ICON_SIZE, height),
                Image::new(self.icon)
                    .fit_to_exact_size(Vec2::splat(dialog::FIELD_ICON_SIZE))
                    .tint(ui.visuals().weak_text_color()),
            )
            .on_hover_text(self.label);
            ui.allocate_ui_with_layout(
                vec2(ui.available_width(), 0.0),
                Layout::top_down(Align::Min),
                body,
            )
            .inner
        });
        ui.add_space(dialog::FIELD_GAP);
        response
    }
}
