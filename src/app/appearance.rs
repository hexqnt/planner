use egui::{FontId, Stroke, Vec2};

use super::{
    BLUE,
    ui_config::{layout, style},
};

/// Толщина линий значка темы.
const THEME_ICON_STROKE: f32 = 1.4;

pub(super) fn apply_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    for (name, weight) in [
        ("Inter", style::FONT_WEIGHT),
        ("Inter semibold", style::SEMIBOLD_WEIGHT),
    ] {
        let font = egui::FontData::from_static(include_bytes!("../../assets/fonts/Inter.ttf"))
            .tweak(egui::FontTweak {
                coords: egui::epaint::text::VariationCoords::new([(b"wght", weight)]),
                ..Default::default()
            });
        fonts
            .font_data
            .insert(name.into(), std::sync::Arc::new(font));
    }
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "Inter".into());
    fonts.families.insert(
        egui::FontFamily::Name("semibold".into()),
        vec!["Inter semibold".into()],
    );
    ctx.set_fonts(fonts);
}

pub(super) fn apply_style(ctx: &egui::Context, dark: bool) {
    ctx.set_theme(if dark {
        egui::Theme::Dark
    } else {
        egui::Theme::Light
    });
    let mut visuals = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    let background = if dark {
        style::DARK_BACKGROUND
    } else {
        style::LIGHT_BACKGROUND
    };
    visuals.widgets.inactive.bg_fill = background;
    visuals.widgets.inactive.weak_bg_fill = background;
    visuals.widgets.inactive.bg_stroke = Stroke::new(
        style::BORDER_WIDTH,
        if dark {
            style::DARK_BORDER
        } else {
            style::LIGHT_BORDER
        },
    );
    visuals.panel_fill = background;
    visuals.window_fill = background;
    visuals.override_text_color = Some(if dark {
        style::DARK_TEXT
    } else {
        style::LIGHT_TEXT
    });
    visuals.selection.bg_fill = BLUE;
    visuals.error_fg_color = style::ERROR;
    visuals.warn_fg_color = style::WARNING;
    visuals.widgets.active.bg_fill = BLUE;
    for widget in [
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.corner_radius = egui::CornerRadius::same(style::CORNER_RADIUS);
    }
    ctx.set_visuals(visuals);
    ctx.global_style_mut(|style| {
        style.spacing.item_spacing = style::ITEM_SPACING;
        style.text_styles.insert(
            egui::TextStyle::Heading,
            FontId::new(
                style::HEADING_FONT_SIZE,
                egui::FontFamily::Name("semibold".into()),
            ),
        );
        style.spacing.button_padding = style::BUTTON_PADDING;
        style.text_styles.insert(
            egui::TextStyle::Body,
            FontId::proportional(style::BODY_FONT_SIZE),
        );
        style.text_styles.insert(
            egui::TextStyle::Button,
            FontId::proportional(style::BUTTON_FONT_SIZE),
        );
    });
}

pub(super) fn panel_frame(ui: &egui::Ui, margin: egui::Margin) -> egui::Frame {
    egui::Frame::new()
        .fill(ui.visuals().panel_fill)
        .inner_margin(margin)
}

/// Высота задаётся до раскладки строки, чтобы крупные виджеты не смещали следующие элементы.
pub(super) fn control_height(ui: &egui::Ui) -> f32 {
    ui.spacing().button_padding.y.mul_add(
        2.0,
        ui.text_style_height(&egui::TextStyle::Body)
            .max(ui.text_style_height(&egui::TextStyle::Button))
            .max(ui.spacing().icon_width),
    )
}

pub(super) fn dropdown_style(style: &mut egui::Style) {
    // Рамка и расширение меняют отступы строки при переходе из состояния без фона.
    for widget in [
        &mut style.visuals.widgets.inactive,
        &mut style.visuals.widgets.hovered,
        &mut style.visuals.widgets.active,
        &mut style.visuals.widgets.open,
    ] {
        widget.bg_stroke = Stroke::NONE;
        widget.expansion = 0.0;
    }
}

pub(super) fn theme_button(ui: &mut egui::Ui, dark: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(layout::THEME_BUTTON_SIZE, egui::Sense::click());
    toolbar_hover(ui, rect, &response);
    let stroke = Stroke::new(THEME_ICON_STROKE, ui.visuals().text_color());
    let center = rect.center();
    if dark {
        ui.painter().circle_stroke(center, 4.0, stroke);
        for step in 0..8_u16 {
            let angle = f32::from(step) * std::f32::consts::FRAC_PI_4;
            let direction = Vec2::angled(angle);
            ui.painter().line_segment(
                [center + direction * 7.0, center + direction * 10.0],
                stroke,
            );
        }
    } else {
        ui.painter().circle_filled(center, 8.0, stroke.color);
        ui.painter()
            .circle_filled(center + Vec2::new(4.0, -3.0), 7.0, ui.visuals().panel_fill);
    }
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn toolbar_hover(ui: &egui::Ui, rect: egui::Rect, response: &egui::Response) {
    if response.hovered() || response.is_pointer_button_down_on() {
        let visuals = ui.style().interact(response);
        ui.painter()
            .rect_filled(rect, style::CORNER_RADIUS, visuals.weak_bg_fill);
    }
}
