use super::{Dialog, Heading, Planner};
use crate::app::icons;

impl Planner {
    pub(super) fn about_dialog(&mut self, ctx: &egui::Context) {
        if !self.about_open {
            return;
        }
        let language = self.language();
        let mut close = false;
        about_backdrop(ctx);
        Dialog::About
            .window(ctx, language.text("О программе", "About"))
            .resizable(false)
            .default_width(320.0)
            .default_pos(ctx.content_rect().center() - egui::vec2(180.0, 150.0))
            .show(ctx, |ui| {
                close = Heading::new("planner", language)
                    .icon(icons::INFO, language.text("О программе", "About"))
                    .wrap(false)
                    .show(ui);
                ui.weak(language.text(
                    "Планирование дней, поездок и проектов.",
                    "Plan your days, trips, and projects.",
                ));
                ui.add_space(8.0);
                ui.separator();
                egui::Grid::new("about_metadata")
                    .num_columns(2)
                    .spacing(egui::vec2(20.0, 10.0))
                    .show(ui, |ui| {
                        for (label, value) in [
                            (
                                language.text("Версия", "Version"),
                                env!("CARGO_PKG_VERSION"),
                            ),
                            (language.text("Автор", "Author"), env!("CARGO_PKG_AUTHORS")),
                            ("holidays-ru", env!("HOLIDAYS_RU_VERSION")),
                        ] {
                            ui.weak(label);
                            ui.label(value);
                            ui.end_row();
                        }
                    });
                ui.separator();
                ui.horizontal(|ui| {
                    let icon = egui::Image::new(icons::GITHUB)
                        .fit_to_exact_size(egui::Vec2::splat(20.0))
                        .tint(ui.visuals().hyperlink_color)
                        .sense(egui::Sense::click());
                    let response = ui
                        .add(icon)
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .on_hover_text(env!("CARGO_PKG_REPOSITORY"));
                    if response.clicked() || response.clicked_with_open_in_background() {
                        ui.open_url(egui::OpenUrl {
                            url: env!("CARGO_PKG_REPOSITORY").into(),
                            new_tab: response.clicked_with_open_in_background(),
                        });
                    }
                    ui.hyperlink_to(
                        language.text("Исходный код на GitHub", "Source code on GitHub"),
                        env!("CARGO_PKG_REPOSITORY"),
                    );
                });
                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), 0.0),
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        close |= ui.button(language.text("Закрыть", "Close")).clicked();
                    },
                );
            });
        self.about_open = !close;
    }
}

fn about_backdrop(ctx: &egui::Context) {
    let backdrop = egui::Area::new(Dialog::About.id().with("modal_overlay"))
        .interactable(true)
        .fixed_pos(ctx.content_rect().min)
        .show(ctx, |ui| {
            let rect = ctx.content_rect();
            ui.allocate_response(rect.size(), egui::Sense::click_and_drag());
            ui.painter().rect_filled(
                rect,
                egui::CornerRadius::ZERO,
                egui::Color32::from_black_alpha(120),
            );
        });
    if ctx.top_layer_id() == Some(backdrop.response.layer_id) {
        ctx.move_to_top(egui::LayerId::new(egui::Order::Middle, Dialog::About.id()));
    }
}
