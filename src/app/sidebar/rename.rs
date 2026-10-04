use crate::app::widgets::TextEditExt as _;

use egui::{Key, Modifiers, RichText};

use crate::{
    app::{
        BLUE, TreeTarget,
        ui_config::{sidebar as config, style},
    },
    text::{Language, Name},
};

/// Внутренние отступы поля переименования календаря или группы.
const RENAME_MARGIN: egui::Margin = egui::Margin::symmetric(4, 0);

pub(in crate::app) struct RenameDraft {
    target: TreeTarget,
    text: String,
    focus: Option<FocusRequest>,
}

enum FocusRequest {
    Cursor,
    SelectAll,
}

impl RenameDraft {
    pub(in crate::app) fn new(target: TreeTarget, name: &str) -> Self {
        Self {
            target,
            text: name.into(),
            focus: Some(FocusRequest::Cursor),
        }
    }

    pub(super) fn selected(target: TreeTarget, name: &str) -> Self {
        Self {
            focus: Some(FocusRequest::SelectAll),
            ..Self::new(target, name)
        }
    }
}

pub(super) struct NameRow<'a> {
    target: TreeTarget,
    name: &'a mut Name,
    enabled: &'a mut bool,
    strong: bool,
}

impl<'a> NameRow<'a> {
    pub(super) const fn new(target: TreeTarget, name: &'a mut Name, enabled: &'a mut bool) -> Self {
        Self {
            target,
            name,
            enabled,
            strong: false,
        }
    }

    pub(super) const fn strong(mut self) -> Self {
        self.strong = true;
        self
    }

    pub(super) fn show(
        self,
        ui: &mut egui::Ui,
        language: Language,
        rename: &mut Option<RenameDraft>,
    ) -> bool {
        ui.spacing_mut().icon_spacing += config::CHECKBOX_TEXT_GAP;
        let Self {
            target,
            name,
            enabled,
            strong,
        } = self;
        let Some(draft) = rename.as_mut().filter(|draft| draft.target == target) else {
            let text = name.get(language);
            let label = RichText::new(text);
            return calendar_checkbox(ui, enabled, if strong { label.strong() } else { label })
                .on_hover_text(text)
                .changed();
        };

        let checkbox = calendar_checkbox(ui, enabled, RichText::new(""));
        let changed = checkbox.changed();
        let margin = RENAME_MARGIN;
        // Текст начинается там же, где подпись checkbox, без дополнительного межвиджетного отступа.
        let left = checkbox.rect.left() + ui.spacing().icon_width + ui.spacing().icon_spacing
            - f32::from(margin.left);
        let editor_rect = egui::Rect::from_min_max(
            egui::pos2(left, checkbox.rect.top()),
            egui::pos2(ui.max_rect().right(), checkbox.rect.bottom()),
        );
        let (escape, enter) = ui.input(|input| {
            (
                input.key_pressed(Key::Escape),
                input.key_pressed(Key::Enter),
            )
        });
        let response = ui.put(
            editor_rect,
            egui::TextEdit::singleline(&mut draft.text)
                .id_salt("rename")
                .font(egui::TextStyle::Body)
                .margin(margin)
                .desired_width(editor_rect.width())
                .min_size(editor_rect.size())
                .char_limit(100)
                .with_context_menu(language),
        );
        if let Some(focus) = draft.focus.take() {
            response.request_focus();
            if matches!(focus, FocusRequest::SelectAll) {
                let mut state = egui::TextEdit::load_state(ui.ctx(), response.id)
                    .expect("Rendered rename editor state");
                state
                    .cursor
                    .set_char_range(Some(egui::text::CCursorRange::two(
                        egui::text::CCursor::new(0),
                        egui::text::CCursor::new(draft.text.chars().count()),
                    )));
                state.store(ui.ctx(), response.id);
                response.scroll_to_me(Some(egui::Align::Center));
                ui.ctx().request_repaint();
            }
        }
        let keyboard_active =
            !egui::Popup::is_any_open(ui.ctx()) && (response.has_focus() || response.lost_focus());
        let lost_focus = response.lost_focus() && !response.has_focus();
        if escape && keyboard_active {
            ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape));
            response.surrender_focus();
            *rename = None;
        } else if (enter && keyboard_active) || lost_focus {
            if enter && keyboard_active {
                ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Enter));
            }
            if let Ok(new_name) = Name::custom(draft.text.trim()) {
                *name = new_name;
                response.surrender_focus();
                *rename = None;
                return true;
            }
            if enter && keyboard_active {
                response.request_focus();
            } else {
                *rename = None;
            }
        }
        changed
    }
}

fn calendar_checkbox(ui: &mut egui::Ui, enabled: &mut bool, label: RichText) -> egui::Response {
    let label = label.color(ui.visuals().text_color());
    ui.scope(|ui| {
        let visuals = ui.visuals_mut();
        for widget in [
            &mut visuals.widgets.inactive,
            &mut visuals.widgets.hovered,
            &mut visuals.widgets.active,
        ] {
            widget.corner_radius = egui::CornerRadius::same(config::CHECKBOX_RADIUS);
            if *enabled {
                widget.bg_fill = BLUE;
                widget.fg_stroke =
                    egui::Stroke::new(config::CHECKBOX_STROKE, style::TEXT_ON_ACCENT);
                widget.bg_stroke = egui::Stroke::NONE;
            }
        }
        ui.checkbox(enabled, label)
    })
    .inner
}

#[cfg(test)]
mod tests;
