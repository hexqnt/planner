use egui::collapsing_header::CollapsingState;

use crate::{
    app::{TreeTarget, dialogs::TreeDraft, icons, ui_config::sidebar as config, widgets},
    model::{Category, CategoryId, Group},
    text::Language,
};

use super::{
    create::CreateTarget,
    rename::{NameRow, RenameDraft},
};
use config::{COLOR_BUTTON_SIZE, ROW_BUTTON_PADDING};

/// Отступ нижнего конца линии дерева от границы группы.
const TREE_LINE_BOTTOM_INSET: f32 = 2.0;

/// Представление заимствует черновики; создание элементов откладывается до окончания обхода дерева.
pub(super) struct TreeView<'a> {
    language: Language,
    draft: &'a mut Option<TreeDraft>,
    rename: &'a mut Option<RenameDraft>,
    create: Option<CreateTarget>,
    event: Option<CategoryId>,
}

pub(super) struct TreeResponse {
    pub(super) changed: bool,
    pub(super) create: Option<CreateTarget>,
    pub(super) event: Option<CategoryId>,
}

impl<'a> TreeView<'a> {
    pub(super) const fn new(
        language: Language,
        draft: &'a mut Option<TreeDraft>,
        rename: &'a mut Option<RenameDraft>,
    ) -> Self {
        Self {
            language,
            draft,
            rename,
            create: None,
            event: None,
        }
    }

    pub(super) fn show(mut self, ui: &mut egui::Ui, groups: &mut [Group]) -> TreeResponse {
        // Отступ компенсирует подъём кнопки добавления группы у верхнего края прокрутки.
        ui.add_space(ui.spacing().button_padding.y);
        let mut changed = false;
        for (index, group) in groups.iter_mut().enumerate() {
            changed |= self.group(ui, index, group);
        }
        if add_group_button(ui, self.language).clicked() {
            self.create = Some(CreateTarget::Group);
        }
        TreeResponse {
            changed,
            create: self.create,
            event: self.event,
        }
    }

    fn group(&mut self, ui: &mut egui::Ui, index: usize, group: &mut Group) -> bool {
        let language = self.language;
        let mut changed = false;
        ui.push_id(index, |ui| {
            let id = ui.make_persistent_id("group_header");
            let mut state = CollapsingState::load_with_default_open(ui.ctx(), id, group.expanded);
            state.set_open(group.expanded);
            let menu_id = ui.make_persistent_id("group_menu");
            let show_menu_button = row_menu_visible(ui, menu_id);
            let header = state.show_header(ui, |ui| {
                ui.spacing_mut().button_padding = ROW_BUTTON_PADDING;
                let menu_width = ui.spacing().interact_size.y;
                let controls_width = menu_width + ui.spacing().item_spacing.x;
                changed |= self.name(
                    ui,
                    NameRow::new(
                        TreeTarget::Group(index),
                        &mut group.name,
                        &mut group.enabled,
                    )
                    .strong(),
                    controls_width,
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let response = ui.add_visible(show_menu_button, egui::Button::new("…"));
                    egui::Popup::menu(&response).id(menu_id).show(|ui| {
                        self.menu(ui, TreeTarget::Group(index), group.name.get(language));
                    });
                });
            });
            changed |= group.expanded != header.is_open();
            group.expanded = header.is_open();
            header.body_unindented(|ui| {
                // Центр checkbox группы не совпадает со стандартной линией отступа egui.
                let line_x = ui.next_widget_position().x
                    + ui.spacing().indent
                    + ui.spacing().icon_width / 2.0;
                ui.spacing_mut().indent *= config::TREE_INDENT_FACTOR;
                ui.visuals_mut().indent_has_left_vline = false;
                let body = ui.indent(id, |ui| {
                    for category in &mut group.categories {
                        changed |= self.category(ui, category);
                    }
                });
                if !group.categories.is_empty() {
                    ui.painter().line_segment(
                        [
                            egui::pos2(line_x, body.response.rect.top()),
                            egui::pos2(
                                line_x,
                                body.response.rect.bottom() - TREE_LINE_BOTTOM_INSET,
                            ),
                        ],
                        ui.visuals().widgets.noninteractive.bg_stroke,
                    );
                }
            });
            ui.add_space(config::GROUP_GAP);
            ui.separator();
            ui.add_space(config::GROUP_GAP);
        });
        changed
    }

    fn category(&mut self, ui: &mut egui::Ui, category: &mut Category) -> bool {
        let language = self.language;
        let mut changed = false;
        ui.push_id(category.id.0, |ui| {
            let menu_id = ui.make_persistent_id("category_menu");
            let show_menu_button = row_menu_visible(ui, menu_id);
            ui.horizontal(|ui| {
                ui.spacing_mut().button_padding = ROW_BUTTON_PADDING;
                let controls_width = 2.0_f32.mul_add(
                    ui.spacing().item_spacing.x,
                    COLOR_BUTTON_SIZE + ui.spacing().interact_size.y,
                );
                changed |= self.name(
                    ui,
                    NameRow::new(
                        TreeTarget::Category(category.id),
                        &mut category.name,
                        &mut category.enabled,
                    ),
                    controls_width,
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let response = ui.add_visible(show_menu_button, egui::Button::new("…"));
                    egui::Popup::menu(&response).id(menu_id).show(|ui| {
                        self.menu(
                            ui,
                            TreeTarget::Category(category.id),
                            category.name.get(language),
                        );
                    });
                    changed |= ui
                        .add(widgets::CalendarColorInput::new(
                            &mut category.color,
                            language,
                        ))
                        .changed();
                });
            })
            .response
            .context_menu(|ui| {
                self.menu(
                    ui,
                    TreeTarget::Category(category.id),
                    category.name.get(language),
                );
            });
        });
        changed
    }

    fn name(&mut self, ui: &mut egui::Ui, row: NameRow<'_>, controls_width: f32) -> bool {
        let size = egui::vec2(
            (ui.available_width() - controls_width).max(0.0),
            ui.spacing().interact_size.y,
        );
        ui.allocate_ui_with_layout(
            size,
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                row.show(ui, self.language, self.rename)
            },
        )
        .inner
    }

    fn menu(&mut self, ui: &mut egui::Ui, target: TreeTarget, name: &str) {
        let language = self.language;
        if let TreeTarget::Category(id) = target
            && ui.button(language.text("+ Событие", "+ Event")).clicked()
        {
            self.event = Some(id);
            ui.close();
        }
        if ui
            .add(widgets::ActionButton::new(
                icons::EDIT,
                language.text("Переименовать", "Rename"),
            ))
            .clicked()
        {
            *self.rename = Some(RenameDraft::new(target, name));
            ui.close();
        }
        if let TreeTarget::Group(index) = target
            && ui
                .button(language.text("+ Календарь", "+ Calendar"))
                .clicked()
        {
            self.create = Some(CreateTarget::Category(index));
            ui.close();
        }
        if ui
            .add(widgets::ActionButton::new(
                icons::TRASH,
                language.text("Удалить", "Delete"),
            ))
            .clicked()
        {
            *self.draft = Some(TreeDraft::new(target, name));
            ui.close();
        }
    }
}

fn add_group_button(ui: &mut egui::Ui, language: Language) -> egui::Response {
    let spacing = ui.spacing();
    let header_height = spacing
        .interact_size
        .y
        .max(ui.text_style_height(&egui::TextStyle::Body));
    let button_height = spacing.interact_size.y.max(2.0_f32.mul_add(
        spacing.button_padding.y,
        ui.text_style_height(&egui::TextStyle::Button),
    ));
    // Центр кнопки совпадает с центром заголовка группы, которая появится на её месте.
    ui.add_space((header_height - button_height) / 2.0);
    ui.button(language.text("+ Добавить группу", "+ Add group"))
}

fn row_menu_visible(ui: &egui::Ui, menu_id: egui::Id) -> bool {
    let rect = egui::Rect::from_min_size(
        ui.next_widget_position(),
        egui::vec2(ui.available_width(), ui.spacing().interact_size.y),
    );
    ui.rect_contains_pointer(rect) || egui::Popup::is_id_open(ui.ctx(), menu_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Planner;

    #[test]
    fn add_group_button_and_new_header_share_vertical_center() {
        for language in [Language::Russian, Language::English] {
            let ctx = egui::Context::default();
            let mut planner = Planner::from_document(
                crate::model::Document {
                    language,
                    ..Default::default()
                },
                &ctx,
                None,
            );
            let render_center = |planner: &mut Planner, label: &str| {
                let output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(300.0, 2000.0),
                        )),
                        ..Default::default()
                    },
                    |ui| {
                        egui::CentralPanel::default().show(ui, |ui| planner.sidebar(ui));
                    },
                );
                let center = output
                    .shapes
                    .iter()
                    .find_map(|shape| match &shape.shape {
                        egui::Shape::Text(text) if text.galley.text() == label => {
                            Some(text.pos.y + text.galley.size().y / 2.0)
                        }
                        _ => None,
                    })
                    .expect("Row label is painted");
                output.drop_without_applying_deltas();
                center
            };
            let button = language.text("+ Добавить группу", "+ Add group");
            render_center(&mut planner, button);
            let before = render_center(&mut planner, button);
            planner.create_tree_entry(CreateTarget::Group);
            let after = render_center(&mut planner, language.text("Новая группа", "New group"));
            assert!((before - after).abs() <= 1.0, "{before} != {after}");
        }
    }
}
