use super::{Planner, shortcuts::Shortcut, ui_config::dialog as config};
use crate::text::Language;

pub(super) use event::EventDraft;
pub(super) use tree::TreeDraft;

mod about;
mod backup;
mod event;
mod heading;
mod tree;

pub(super) use heading::Heading;

#[derive(Clone, Copy)]
pub(super) enum Dialog {
    Vacation,
    Event,
    Tree,
    Backup,
    Notice,
    About,
    #[cfg(target_arch = "wasm32")]
    Import,
}

impl Dialog {
    pub(super) fn window<'a>(self, ctx: &egui::Context, title: &'a str) -> egui::Window<'a> {
        egui::Window::new(title)
            .id(self.id())
            .collapsible(false)
            .title_bar(false)
            .frame(window_frame(ctx))
    }

    pub(super) fn id(self) -> egui::Id {
        egui::Id::new(match self {
            Self::Vacation => "vacation_calculator",
            Self::Event => "event_editor",
            Self::Tree => "tree_editor",
            Self::Backup => "backup_editor",
            Self::Notice => "notice",
            Self::About => "about",
            #[cfg(target_arch = "wasm32")]
            Self::Import => "import_calendar_dialog",
        })
    }

    fn from_id(id: egui::Id) -> Option<Self> {
        [
            Self::Vacation,
            Self::Event,
            Self::Tree,
            Self::Backup,
            Self::Notice,
            Self::About,
            #[cfg(target_arch = "wasm32")]
            Self::Import,
        ]
        .into_iter()
        .find(|dialog| dialog.id() == id)
    }
}

pub(super) fn close_button(ui: &mut egui::Ui, language: Language) -> egui::Response {
    ui.add_sized(
        config::CLOSE_BUTTON_SIZE,
        egui::Button::new("×").frame(false),
    )
    .on_hover_text(language.text("Закрыть", "Close"))
}

pub(super) fn window_frame(ctx: &egui::Context) -> egui::Frame {
    egui::Frame::window(&ctx.global_style())
        .inner_margin(config::WINDOW_MARGIN)
        .corner_radius(config::WINDOW_RADIUS)
}

pub(super) fn header(
    ui: &mut egui::Ui,
    language: Language,
    contents: impl FnOnce(&mut egui::Ui),
) -> bool {
    header_sized(ui, language, ui.spacing().interact_size.y, contents)
}

pub(super) fn header_sized(
    ui: &mut egui::Ui,
    language: Language,
    height: f32,
    contents: impl FnOnce(&mut egui::Ui),
) -> bool {
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), height),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            contents(ui);
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), config::CLOSE_BUTTON_SIZE.y),
                egui::Layout::right_to_left(egui::Align::Center),
                |ui| close_button(ui, language).clicked(),
            )
            .inner
        },
    )
    .inner
}

impl Planner {
    pub(super) fn dialogs(&mut self, ctx: &egui::Context) {
        self.close_focused_dialog(ctx);
        self.vacation_window(ctx);
        self.event_dialog(ctx);
        self.tree_dialog(ctx);
        self.backup_dialog(ctx);
        self.about_dialog(ctx);
        if let Some(notice) = &self.notice {
            let mut close = false;
            let language = self.language();
            let title = language.text("Сообщение", "Message");
            Dialog::Notice
                .window(ctx, title)
                .resizable(false)
                .show(ctx, |ui| {
                    close = Heading::new(title, language).show(ui);
                    ui.label(notice);
                    close |= ui.button("OK").clicked();
                });
            if close {
                self.notice = None;
            }
        }
    }

    fn close_focused_dialog(&mut self, ctx: &egui::Context) {
        if self.shortcuts.popup_open(ctx) {
            return;
        }
        let Some(layer) = ctx.top_layer_id() else {
            return;
        };
        let Some(dialog) = Dialog::from_id(layer.id) else {
            return;
        };
        if !self.shortcuts.consume(Shortcut::Cancel, ctx) {
            return;
        }
        match dialog {
            Dialog::Vacation => self.close_vacation(),
            Dialog::Event => self.event_draft = None,
            Dialog::Tree => self.tree_draft = None,
            Dialog::Backup => self.backup = None,
            Dialog::Notice => self.notice = None,
            Dialog::About => self.about_open = false,
            #[cfg(target_arch = "wasm32")]
            Dialog::Import => self.files.cancel_import(),
        }
    }
}

#[cfg(feature = "testing")]
pub(super) fn event_rect(ctx: &egui::Context) -> Option<egui::Rect> {
    ctx.memory(|memory| memory.area_rect(Dialog::Event.id()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{app::TreeTarget, model::Document};
    use egui::Vec2;

    fn dialog_frame(ctx: &egui::Context, planner: &mut Planner, escape: bool) {
        let events = if escape {
            [true, false]
                .into_iter()
                .map(|pressed| egui::Event::Key {
                    key: egui::Key::Escape,
                    physical_key: None,
                    pressed,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                })
                .collect()
        } else {
            Vec::new()
        };
        ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    Vec2::new(1200.0, 900.0),
                )),
                events,
                ..Default::default()
            },
            |ui| planner.dialogs(ui.ctx()),
        )
        .drop_without_applying_deltas();
    }

    #[test]
    fn escape_closes_only_the_focused_dialog_without_changing_document() {
        for focused in [
            "event_editor",
            "tree_editor",
            "backup_editor",
            "notice",
            "about",
        ] {
            let ctx = egui::Context::default();
            let mut planner = Planner::from_document(Document::default(), &ctx, None);
            planner.new_event();
            planner.tree_draft = Some(TreeDraft::new(TreeTarget::Group(0), "Work"));
            planner.backup = Some(String::new());
            planner.notice = Some("Test notice".into());
            planner.about_open = true;
            let document = serde_json::to_string(&planner.document).unwrap();
            dialog_frame(&ctx, &mut planner, false);
            ctx.move_to_top(egui::LayerId::new(
                egui::Order::Middle,
                egui::Id::new(focused),
            ));
            dialog_frame(&ctx, &mut planner, false);
            assert_eq!(ctx.top_layer_id().unwrap().id, egui::Id::new(focused));

            dialog_frame(&ctx, &mut planner, true);

            assert_eq!(planner.event_draft.is_none(), focused == "event_editor");
            assert_eq!(planner.tree_draft.is_none(), focused == "tree_editor");
            assert_eq!(planner.backup.is_none(), focused == "backup_editor");
            assert_eq!(planner.notice.is_none(), focused == "notice");
            assert_eq!(!planner.about_open, focused == "about");
            assert_eq!(serde_json::to_string(&planner.document).unwrap(), document);
        }
    }

    #[test]
    fn escape_is_left_for_an_unrelated_window() {
        let ctx = egui::Context::default();
        let mut planner = Planner::from_document(Document::default(), &ctx, None);
        let id = egui::Id::new("unrelated_window");
        ctx.run_ui(egui::RawInput::default(), |ui| {
            egui::Window::new("Other window")
                .id(id)
                .show(ui.ctx(), |ui| {
                    ui.label("Other content");
                });
        })
        .drop_without_applying_deltas();
        ctx.move_to_top(egui::LayerId::new(egui::Order::Middle, id));
        assert_eq!(ctx.top_layer_id().unwrap().id, id);

        dialog_frame(&ctx, &mut planner, true);

        assert!(ctx.input(|input| input.key_pressed(egui::Key::Escape)));
    }

    #[test]
    fn escape_is_left_for_an_open_popup() {
        let ctx = egui::Context::default();
        let mut planner = Planner::from_document(Document::default(), &ctx, None);
        planner.new_event();
        dialog_frame(&ctx, &mut planner, false);
        let popup = egui::Id::new("test_popup");
        egui::Popup::open_id(&ctx, popup);

        dialog_frame(&ctx, &mut planner, true);

        assert!(planner.event_draft.is_some());
        assert!(ctx.input(|input| input.key_pressed(egui::Key::Escape)));
        egui::Popup::close_id(&ctx, popup);
        dialog_frame(&ctx, &mut planner, true);
        assert!(planner.event_draft.is_none());
    }
}
