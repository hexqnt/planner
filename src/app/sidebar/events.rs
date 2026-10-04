use std::ops::Range;

use egui::{Rect, TextStyle, Ui, Vec2};

use crate::model::EventId;

use super::super::{Planner, widgets};

/// Дополнительная высота строки списка событий поверх текста и стандартных интервалов.
const EVENT_ROW_EXTRA_HEIGHT: f32 = 5.0;

/// Геометрия списка сохраняет полный размер прокрутки, но выделяет только видимые строки.
struct RowViewport {
    rect: Rect,
    row_height: f32,
    visible: Range<usize>,
}

impl RowViewport {
    // Преобразования между индексами и координатами соответствуют модели геометрии egui.
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    fn new(ui: &mut Ui, count: usize) -> Self {
        let spacing = ui.spacing();
        let line_height = ui.text_style_height(&TextStyle::Body);
        let title_height = spacing
            .interact_size
            .y
            .max(2.0_f32.mul_add(spacing.button_padding.y, line_height));
        let row_height = 2.0_f32.mul_add(line_height + spacing.item_spacing.y, title_height)
            + EVENT_ROW_EXTRA_HEIGHT;
        let (_, rect) =
            ui.allocate_space(Vec2::new(ui.available_width(), count as f32 * row_height));
        let clip = ui.clip_rect();
        let first = (((clip.top() - rect.top()) / row_height).floor().max(0.0) as usize).min(count);
        let last =
            (((clip.bottom() - rect.top()) / row_height).ceil().max(0.0) as usize).min(count);
        Self {
            rect,
            row_height,
            visible: first..last,
        }
    }

    #[allow(clippy::cast_precision_loss)]
    fn row_rect(&self, index: usize) -> Rect {
        Rect::from_min_size(
            self.rect.min + Vec2::new(0.0, index as f32 * self.row_height),
            Vec2::new(self.rect.width(), self.row_height),
        )
    }
}

impl Planner {
    pub(super) fn event_list(&mut self, ui: &mut egui::Ui) -> Option<EventId> {
        let language = self.language();
        let range = self
            .selection
            .range()
            .unwrap_or_else(|| self.document.year.range());
        let rows = self.event_index.list(&self.document, range);
        let count = rows.len();
        if count == 0 {
            ui.weak(language.text("В этом интервале нет событий.", "No events in this range."));
            return None;
        }
        let viewport = RowViewport::new(ui, count);
        let mut edit = None;
        for (position, entry) in rows
            .enumerate()
            .take(viewport.visible.end)
            .skip(viewport.visible.start)
        {
            let event = entry.event;
            let row = entry.occurrence;
            let row_rect = viewport.row_rect(position);
            let mut row_ui = ui.new_child(egui::UiBuilder::new().max_rect(row_rect));
            row_ui.push_id((event.id.0, row.start()), |ui| {
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                ui.horizontal(|ui| {
                    ui.label(widgets::calendar_marker(row.color));
                    if ui
                        .selectable_label(false, event.title.get())
                        .on_hover_text(event.title.get())
                        .clicked()
                    {
                        edit = Some(event.id);
                    }
                });
                ui.weak(row.date_label()).on_hover_text(row.date_label());
                if !row.visible {
                    ui.weak(language.text("Скрыто на календаре", "Hidden on calendar"));
                }
            });
        }
        edit
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Document;

    #[test]
    fn event_list_only_lays_out_visible_rows_and_keeps_full_scroll_extent() {
        let ctx = egui::Context::default();
        let mut document = Document {
            year: crate::model::Year::try_from(2026).unwrap(),
            ..Document::default()
        };
        for _ in 0..100 {
            document.add_examples();
        }
        let mut planner = Planner::from_document(document, &ctx, None);
        for offset in [0.0, 10_000.0] {
            let output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(260.0, 300.0),
                    )),
                    ..Default::default()
                },
                |ui| {
                    egui::ScrollArea::vertical()
                        .vertical_scroll_offset(offset)
                        .show(ui, |ui| {
                            let before = ui.next_widget_position().y;
                            planner.event_list(ui);
                            assert!(ui.next_widget_position().y - before > 50_000.0);
                        });
                },
            );
            let dates = output.shapes.iter().filter(|shape| {
                matches!(&shape.shape, egui::Shape::Text(text) if text.galley.text().contains(" — "))
            }).count();
            assert!(
                dates > 0 && dates < 10,
                "{dates} date labels at offset {offset}"
            );
            output.drop_without_applying_deltas();
        }
    }

    #[test]
    fn hidden_events_remain_available_in_the_event_list() {
        let ctx = egui::Context::default();
        let mut document = Document::default();
        document.add_examples();
        for group in &mut document.groups {
            group.enabled = false;
        }
        let mut planner = Planner::from_document(document, &ctx, None);
        let output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(500.0, 2000.0),
                )),
                ..Default::default()
            },
            |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    planner.event_list(ui);
                });
            },
        );
        for event in &planner.document.events {
            assert!(output.shapes.iter().any(|shape| {
                matches!(&shape.shape, egui::Shape::Text(text) if text.galley.text() == event.title.get())
            }));
        }
        output.drop_without_applying_deltas();
    }
}
