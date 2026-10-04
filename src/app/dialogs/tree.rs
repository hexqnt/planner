use crate::app::TreeTarget;

use super::{Dialog, Heading, Planner};

pub(in crate::app) struct TreeDraft {
    target: TreeTarget,
    name: String,
}

impl TreeDraft {
    pub(in crate::app) fn new(target: TreeTarget, name: &str) -> Self {
        Self {
            target,
            name: name.into(),
        }
    }

    fn fields(&self, ui: &mut egui::Ui, language: crate::text::Language) {
        ui.label(format!(
            "{} «{}»?",
            language.text("Удалить", "Delete"),
            self.name
        ));
        ui.label(if matches!(self.target, TreeTarget::Group(_)) {
            language.text(
                "Все вложенные календари и их события будут удалены.",
                "All calendars inside and their events will be deleted.",
            )
        } else {
            language.text(
                "Все события этого календаря будут удалены.",
                "All events in this calendar will be deleted.",
            )
        });
    }
}

impl Planner {
    pub(super) fn tree_dialog(&mut self, ctx: &egui::Context) {
        let language = self.language();
        let Some(draft) = &self.tree_draft else {
            return;
        };
        let mut save = false;
        let mut cancel = false;
        let title = language.text("Группы и календари", "Groups and calendars");
        Dialog::Tree
            .window(ctx, title)
            .resizable(false)
            .show(ctx, |ui| {
                cancel = Heading::new(title, language).show(ui);
                draft.fields(ui, language);
                ui.horizontal(|ui| {
                    save = ui
                        .button(language.text("Удалить навсегда", "Delete permanently"))
                        .clicked();
                    cancel |= ui.button(language.text("Отмена", "Cancel")).clicked();
                });
            });
        if save {
            let draft = self.tree_draft.take().expect("Open tree editor");
            match draft.target {
                TreeTarget::Group(index) => self.document.remove_group(index),
                TreeTarget::Category(id) => self.document.remove_category(id),
            }
            self.event_draft = None;
            self.rename_draft = None;
            self.mark_changed();
            ctx.request_repaint();
        } else if cancel {
            self.tree_draft = None;
        }
    }
}
