//! Локальное состояние интерфейса чата; переписка не изменяет документ календаря.

use egui::RichText;

use super::{
    appearance, icons,
    shortcuts::{Shortcut, Shortcuts},
    ui_config::{chat as config, style},
    widgets::{ActionButton, TextEditExt as _},
};
use crate::text::Language;

#[derive(Default)]
pub(super) struct Chat {
    input: String,
    messages: Vec<String>,
}

impl Chat {
    pub(super) fn show(
        &mut self,
        ui: &mut egui::Ui,
        language: Language,
        shortcuts: &mut Shortcuts,
    ) {
        ui.spacing_mut().interact_size.y = appearance::control_height(ui);
        if ui
            .add_enabled(
                !self.messages.is_empty() || !self.input.is_empty(),
                egui::Button::new(language.text("Новый чат", "New chat")),
            )
            .clicked()
        {
            self.messages.clear();
            self.input.clear();
        }
        ui.weak(language.text("AI пока не подключён.", "AI is not connected yet."));
        ui.separator();
        self.composer(ui, language, shortcuts);
        egui::ScrollArea::vertical()
            .id_salt("chat_history")
            .stick_to_bottom(true)
            .auto_shrink([false, false])
            .show(ui, |ui| self.history(ui, language));
    }

    fn composer(&mut self, ui: &mut egui::Ui, language: Language, shortcuts: &mut Shortcuts) {
        let id = ui.make_persistent_id("chat_input");
        let send_key = ui.memory(|memory| memory.has_focus(id))
            && shortcuts.consume(Shortcut::SendChat, ui.ctx());
        let send = Self::controls(ui, language, self.can_send()) || send_key;
        self.input_field(ui, language, id);
        if send && self.can_send() {
            self.messages.push(std::mem::take(&mut self.input));
            ui.memory_mut(|memory| memory.request_focus(id));
        }
    }

    fn can_send(&self) -> bool {
        !self.input.trim().is_empty()
    }

    fn controls(ui: &mut egui::Ui, language: Language, can_send: bool) -> bool {
        let mut send = false;
        egui::Panel::bottom("chat_controls")
            .resizable(false)
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let size = egui::Vec2::splat(appearance::control_height(ui));
                        let button = |icon, label| {
                            ActionButton::new(icon, label)
                                .icon_only(style::ACTION_ICON_SIZE)
                                .min_size(size)
                        };
                        send |= ui
                            .add_enabled(
                                can_send,
                                button(icons::PLAYER_PLAY, language.text("Отправить", "Send")),
                            )
                            .clicked();
                        ui.add_enabled(
                            false,
                            button(icons::PLAYER_STOP, language.text("Остановить", "Stop")),
                        )
                        .on_disabled_hover_text(language.text(
                            "Остановка станет доступна во время ответа AI.",
                            "Stop will be available while AI is responding.",
                        ));
                    });
                });
                ui.add(
                    egui::Label::new(
                        RichText::new(language.text(
                            "Enter — отправить · Shift+Enter — новая строка",
                            "Enter to send · Shift+Enter for a new line",
                        ))
                        .small()
                        .weak(),
                    )
                    .wrap(),
                );
            });
        send
    }

    fn input_field(&mut self, ui: &mut egui::Ui, language: Language, id: egui::Id) {
        let label = language.text("Спросить о календаре…", "Ask about your calendar…");
        let max_height = (ui.available_height() * config::INPUT_MAX_HEIGHT_FRACTION)
            .max(config::INPUT_MIN_HEIGHT);
        egui::Panel::bottom("chat_composer")
            .resizable(true)
            .default_size(config::INPUT_DEFAULT_HEIGHT)
            .min_size(config::INPUT_MIN_HEIGHT)
            .max_size(max_height)
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                let height = ui.available_height();
                egui::ScrollArea::vertical()
                    .id_salt("chat_input_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let response = ui.add_sized(
                            egui::vec2(ui.available_width(), height),
                            egui::TextEdit::multiline(&mut self.input)
                                .id(id)
                                .desired_rows(1)
                                .hint_text(label)
                                .with_context_menu(language),
                        );
                        ui.ctx()
                            .accesskit_node_builder(response.id, |node| node.set_label(label));
                    });
            });
    }

    fn history(&self, ui: &mut egui::Ui, language: Language) {
        if self.messages.is_empty() {
            ui.label(
                RichText::new(language.text("Помощник по календарю", "Calendar assistant"))
                    .strong(),
            );
            for text in [
                language.text(
                    "Здесь можно будет планировать события и задавать вопросы о календаре.",
                    "Plan events and ask questions about your calendar here.",
                ),
                language.text(
                    "Например: «Запланируй встречу на завтра» или «Какие события на этой неделе?».",
                    "Try “Schedule a meeting for tomorrow” or “What events are coming up this week?”.",
                ),
            ] {
                ui.add(egui::Label::new(text).wrap());
            }
            return;
        }
        for (index, message) in self.messages.iter().enumerate() {
            ui.push_id(index, |ui| {
                egui::Frame::new()
                    .fill(ui.visuals().faint_bg_color)
                    .corner_radius(config::MESSAGE_RADIUS)
                    .inner_margin(config::MESSAGE_MARGIN)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(RichText::new(language.text("ВЫ", "YOU")).small().strong());
                        ui.add(egui::Label::new(message).selectable(true).wrap());
                    });
            });
            ui.add_space(config::MESSAGE_GAP);
        }
    }
}
