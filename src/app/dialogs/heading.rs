use egui::{ImageSource, TextWrapMode, Ui};

use crate::{
    app::{ui_config::style, widgets::Help},
    text::Language,
};

use super::{config, header_sized};

pub(in crate::app) struct Heading<'a> {
    title: &'a str,
    language: Language,
    help: Option<Help<'a>>,
    icon: Option<(ImageSource<'a>, &'a str)>,
    font_size: f32,
    wrap_mode: TextWrapMode,
}

impl<'a> Heading<'a> {
    pub(in crate::app) const fn new(title: &'a str, language: Language) -> Self {
        Self {
            title,
            language,
            help: None,
            icon: None,
            font_size: config::HEADING_FONT_SIZE,
            wrap_mode: TextWrapMode::Wrap,
        }
    }

    pub(in crate::app) const fn help(mut self, label: &'a str, text: &'a str) -> Self {
        self.help = Some(Help::new(label, text));
        self
    }

    pub(in crate::app) fn icon(mut self, source: ImageSource<'a>, label: &'a str) -> Self {
        self.icon = Some((source, label));
        self
    }

    pub(in crate::app) const fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    pub(in crate::app) const fn wrap(mut self, wrap: bool) -> Self {
        self.wrap_mode = if wrap {
            TextWrapMode::Wrap
        } else {
            TextWrapMode::Truncate
        };
        self
    }

    pub(in crate::app) fn show(self, ui: &mut Ui) -> bool {
        let spacing = ui.spacing().item_spacing.x;
        let mut width = ui.available_width() - config::CLOSE_BUTTON_SIZE.x - spacing;
        if self.icon.is_some() {
            width -= self.font_size + spacing;
        }
        if self.help.is_some() {
            width -= style::ACTION_ICON_SIZE.x + spacing;
        }
        let title = egui::WidgetText::from(egui::RichText::new(self.title).size(self.font_size))
            .into_galley(
                ui,
                Some(self.wrap_mode),
                width.max(1.0),
                egui::TextStyle::Heading,
            );
        // Все элементы размещаются после расчёта высоты текста, включая переносы на узких окнах.
        let mut height = title.size().y.max(config::CLOSE_BUTTON_SIZE.y);
        if self.icon.is_some() {
            height = height.max(self.font_size);
        }
        if self.help.is_some() {
            height = height.max(style::ACTION_ICON_SIZE.y);
        }
        let close = header_sized(ui, self.language, height, |ui| {
            if let Some((source, label)) = self.icon {
                let response = ui.add(
                    egui::Image::new(source)
                        .fit_to_exact_size(egui::Vec2::splat(self.font_size))
                        .tint(ui.visuals().text_color()),
                );
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Image, response.enabled(), label)
                });
            }
            ui.add(egui::Label::new(title));
            if let Some(help) = self.help {
                ui.add(help);
            }
        });
        ui.add_space(config::HEADER_GAP);
        close
    }
}
