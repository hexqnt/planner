use egui::{Response, Ui};

/// Набор вариантов и функция подписи мономорфизируются для каждого типа выбора.
pub(in crate::app) fn choice<T: Copy + PartialEq, const N: usize>(
    ui: &mut Ui,
    id: impl egui::AsIdSalt,
    selected: &mut T,
    values: [T; N],
    label: impl Fn(T) -> &'static str,
) -> Response {
    let before = *selected;
    let font = egui::TextStyle::Button.resolve(ui.style());
    let width = values
        .iter()
        .copied()
        .map(|value| {
            ui.painter()
                .layout_no_wrap(label(value).into(), font.clone(), ui.visuals().text_color())
                .size()
                .x
        })
        .fold(0.0_f32, f32::max);
    let mut response = egui::ComboBox::from_id_salt(id)
        .width(
            ui.spacing()
                .button_padding
                .x
                .mul_add(2.0, width + ui.spacing().icon_width),
        )
        .popup_style(crate::app::appearance::dropdown_style.into())
        .selected_text(label(*selected))
        .show_ui(ui, |ui| {
            for value in values {
                ui.selectable_value(selected, value, label(value));
            }
        })
        .response;
    if before != *selected {
        response.mark_changed();
    }
    response
}
