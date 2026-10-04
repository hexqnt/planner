use crate::app::widgets::TextEditExt as _;

use crate::{app::ui_config::backup as config, model::Document};

use super::{Dialog, Heading, Planner};

impl Planner {
    pub(super) fn backup_dialog(&mut self, ctx: &egui::Context) {
        let language = self.language();
        let Some(json) = &mut self.backup else {
            return;
        };
        let mut close = false;
        let mut imported = None;
        let title = language.text("Резервная копия JSON", "JSON backup");
        Dialog::Backup.window(ctx, title)
            .default_size(config::WINDOW_SIZE)
            .show(ctx, |ui| {
                close = Heading::new(title, language)
                    .help(
                        language.text("О резервной копии", "About JSON backup"),
                        language.text(
                            "Скопируйте JSON в файл для резервной копии. Для восстановления вставьте его сюда и нажмите «Заменить календарь». Текущие данные будут заменены.",
                            "Copy the JSON to a file for backup. To restore, paste it here and click ‘Replace calendar’. Current data will be replaced.",
                        ),
                    )
                    .show(ui);
                ui.weak(language.text("При восстановлении текущие данные будут заменены.", "Restoring replaces current data."));
                egui::ScrollArea::vertical()
                    .max_height(config::EDITOR_MAX_HEIGHT)
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(json)
                                .code_editor()
                                .desired_width(f32::INFINITY)
                                .desired_rows(config::EDITOR_ROWS)
                                .with_context_menu(language),
                        );
                    });
                ui.horizontal(|ui| {
                    if ui.button(language.text("Копировать JSON", "Copy JSON")).clicked() {
                        ctx.copy_text(json.clone());
                    }
                    if ui.button(language.text("Заменить календарь", "Replace calendar")).clicked() {
                        match Document::parse(json) {
                            Ok(document) => imported = Some(document),
                            Err(error) => self.notice = Some(format!("Unable to import calendar: {error}")),
                        }
                    }
                });
            });
        if let Some(document) = imported {
            self.replace_document(document, ctx);
        } else if close {
            self.backup = None;
        }
    }
}
