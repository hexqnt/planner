use egui::{Button, Image, ImageSource, InnerResponse, Response, Ui, Vec2, Widget};

use crate::app::ui_config::style;

use super::labelled_action;

/// Кнопка действия сохраняет доступную подпись и подсказку даже без видимого текста.
pub(in crate::app) struct ActionButton<'a> {
    image: Image<'a>,
    label: &'a str,
    icon_only: bool,
    min_size: Vec2,
}

impl<'a> ActionButton<'a> {
    pub(in crate::app) fn new(icon: ImageSource<'a>, label: &'a str) -> Self {
        Self {
            image: Image::new(icon).fit_to_exact_size(style::ACTION_ICON_SIZE),
            label,
            icon_only: false,
            min_size: Vec2::ZERO,
        }
    }

    pub(in crate::app) fn icon_only(mut self, size: Vec2) -> Self {
        self.image = self.image.fit_to_exact_size(size);
        self.icon_only = true;
        self
    }

    pub(in crate::app) const fn min_size(mut self, size: Vec2) -> Self {
        self.min_size = size;
        self
    }

    pub(in crate::app) fn show_menu<R>(
        self,
        ui: &mut Ui,
        contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<Option<R>> {
        let label = self.label;
        let (response, inner) =
            egui::containers::menu::MenuButton::from_button(self.button()).ui(ui, contents);
        InnerResponse {
            response: labelled_action(response, label),
            inner: inner.map(|contents| contents.inner),
        }
    }

    fn button(self) -> Button<'a> {
        let button = if self.icon_only {
            Button::new(self.image).frame_when_inactive(false)
        } else {
            Button::image_and_text(self.image, self.label)
        };
        button
            .image_tint_follows_text_color(true)
            .min_size(self.min_size)
    }
}

impl Widget for ActionButton<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let label = self.label;
        labelled_action(ui.add(self.button()), label)
    }
}
