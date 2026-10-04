use egui::{Image, Response, Ui, Widget};

use crate::app::{icons, ui_config::style};

/// Отдельная иконка для пояснений, относящихся ко всему окну.
pub(in crate::app) struct Help<'a> {
    label: &'a str,
    text: &'a str,
}

impl<'a> Help<'a> {
    pub(in crate::app) const fn new(label: &'a str, text: &'a str) -> Self {
        Self { label, text }
    }
}

impl Widget for Help<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let response = ui.add(
            Image::new(icons::HELP)
                .fit_to_exact_size(style::ACTION_ICON_SIZE)
                .tint(ui.visuals().weak_text_color()),
        );
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Image, response.enabled(), self.label)
        });
        super::hint(response, self.text)
    }
}
