use crate::app::{Planner, widgets};

impl Planner {
    pub(super) fn year_picker(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        let mut year = self.document.year;
        let language = self.language();
        let response = ui
            .scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                ui.add(widgets::YearInput::new(
                    &mut year,
                    &mut self.year_draft,
                    language,
                ))
            })
            .inner;
        if response.changed() {
            self.change_year(year);
        }
    }
}

#[cfg(test)]
mod tests;
