use egui::{Color32, Key, Modifiers, TextFormat};
use unicode_segmentation::UnicodeSegmentation as _;

use super::Planner;
use crate::app::{icons, sidebar::SidebarTab, ui_config::style, widgets};
use crate::model::{EventId, HighlightField};

#[cfg(test)]
mod tests;

const ROW_HEIGHT: f32 = 148.0;

enum SearchAction {
    Navigate(usize),
    Edit(EventId),
}

impl Planner {
    pub(super) fn search_results(&mut self, ui: &mut egui::Ui, overlay: bool) {
        let language = self.language();
        let count = self.search.index.results().len();
        ui.weak(format!("{}: {count}", language.text("Найдено", "Found")));
        if count == 0 {
            ui.label(language.text(
                "Совпадений нет. Попробуйте другой запрос или расширьте фильтры.",
                "No matches. Try another query or broaden the filters.",
            ));
            return;
        }
        let mut action = self.search.activate.take().map(SearchAction::Navigate);
        let mut scroll = egui::ScrollArea::vertical().id_salt("search_results");
        if self.search.scroll_selection {
            #[expect(
                clippy::cast_precision_loss,
                reason = "Индекс строки переводится в координату прокрутки UI."
            )]
            let offset = self.search.selected as f32 * (ROW_HEIGHT + ui.spacing().item_spacing.y);
            scroll = scroll.vertical_scroll_offset(offset);
            self.search.scroll_selection = false;
        }
        scroll.max_height(ui.available_height().max(0.0)).show_rows(
            ui,
            ROW_HEIGHT,
            count,
            |ui, rows| {
                for position in rows {
                    if let Some(row_action) = self.search_row(ui, position) {
                        action = Some(row_action);
                    }
                }
            },
        );
        match action {
            Some(SearchAction::Navigate(position)) => {
                let dates = self.search.index.results()[position]
                    .schedule
                    .occupied_dates();
                if let Err(error) = self.go_to_date(dates.start()) {
                    self.notice = Some(error.message(language).into());
                } else {
                    self.selection.set_range(dates);
                    if overlay {
                        self.sidebar_tab = SidebarTab::Calendars;
                    } else {
                        self.search.focus = true;
                    }
                }
            }
            Some(SearchAction::Edit(event)) => {
                if overlay {
                    self.sidebar_tab = SidebarTab::Calendars;
                }
                self.edit_event(event);
            }
            None => {}
        }
    }
    pub(super) fn search_keyboard(&mut self, ui: &egui::Ui) {
        let count = self.search.index.results().len();
        if count == 0
            || self.search.index.is_pending()
            || self.search.error.is_some()
            || self.search.query.is_err()
        {
            return;
        }
        let navigation_enabled = !self.shortcuts_blocked()
            && !egui::Popup::is_any_open(ui.ctx())
            && (ui.ctx().memory(|memory| {
                memory
                    .focused()
                    .is_none_or(|id| id == egui::Id::new("event_search_input"))
            }));
        if navigation_enabled {
            ui.input_mut(|input| {
                if input.consume_key(Modifiers::NONE, Key::ArrowDown) {
                    self.search.selected = (self.search.selected + 1).min(count - 1);
                    self.search.scroll_selection = true;
                }
                if input.consume_key(Modifiers::NONE, Key::ArrowUp) {
                    self.search.selected = self.search.selected.saturating_sub(1);
                    self.search.scroll_selection = true;
                }
                if input.consume_key(Modifiers::NONE, Key::Enter) {
                    self.search.activate = Some(self.search.selected);
                }
            });
        }
    }

    fn search_row(&mut self, ui: &mut egui::Ui, position: usize) -> Option<SearchAction> {
        let language = self.language();
        let mut action = None;
        let query = self.search.query.as_ref().expect("Parsed query");
        self.search.index.highlight(
            query,
            position,
            HighlightField::Title,
            &mut self.search.title_indices,
        );
        self.search.index.highlight(
            query,
            position,
            HighlightField::Snippet,
            &mut self.search.snippet_indices,
        );
        let hit = &self.search.index.results()[position];
        let event = hit.source(&self.document);
        let category = self
            .document
            .category(event.category)
            .expect("Known calendar");
        let visible = self.document.visible_category(event.category).is_some();
        let row = egui::Frame::new()
            .inner_margin(egui::Margin::symmetric(6, 4))
            .fill(if self.search.selected == position {
                ui.visuals().faint_bg_color
            } else {
                Color32::TRANSPARENT
            });
        ui.allocate_ui(egui::vec2(ui.available_width(), ROW_HEIGHT), |ui| {
            row.show(ui, |ui| {
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                ui.horizontal(|ui| {
                    ui.label(widgets::calendar_marker(category.color));
                    let title =
                        highlight_job(ui, event.title.get(), &self.search.title_indices, false);
                    let controls = ui
                        .spacing()
                        .button_padding
                        .x
                        .mul_add(2.0, style::INLINE_ICON_SIZE.x)
                        + ui.spacing().item_spacing.x;
                    let width = (ui.available_width() - controls).max(30.0);
                    if ui
                        .add_sized(
                            [width, 26.0],
                            egui::Button::selectable(self.search.selected == position, title),
                        )
                        .on_hover_text(event.title.get())
                        .clicked()
                    {
                        self.search.selected = position;
                        action = Some(SearchAction::Navigate(position));
                    }
                    let edit = ui.add(
                        widgets::ActionButton::new(
                            icons::EDIT,
                            language.text("Редактировать", "Edit"),
                        )
                        .icon_only(style::INLINE_ICON_SIZE),
                    );
                    if edit.clicked() {
                        self.search.selected = position;
                        action = Some(SearchAction::Edit(event.id));
                    }
                });
                ui.weak(hit.date_label()).on_hover_text(hit.date_label());
                ui.weak(category.name.get(language));
                if let Some(snippet) = self.search.index.snippet(position, &self.document) {
                    ui.label(highlight_job(
                        ui,
                        snippet,
                        &self.search.snippet_indices,
                        true,
                    ));
                }
                ui.horizontal(|ui| {
                    if event.schedule.recurrence().is_some() {
                        ui.weak(language.text("Повторяется", "Repeats"));
                    }
                    if !visible {
                        ui.weak(language.text("Скрыто на календаре", "Hidden on calendar"));
                    }
                });
            });
        });
        action
    }
}

/// Индексы nucleo относятся к графемам; подсветка сохраняет UTF-8 границы и комбинируемые символы.
fn highlight_job(
    ui: &egui::Ui,
    text: &str,
    indices: &[u32],
    snippet: bool,
) -> egui::text::LayoutJob {
    let first = indices.first().copied().unwrap_or(0) as usize;
    let start = if snippet { first.saturating_sub(24) } else { 0 };
    let limit = if snippet { 100 } else { usize::MAX };
    let normal = TextFormat {
        font_id: egui::TextStyle::Body.resolve(ui.style()),
        color: ui.visuals().text_color(),
        ..TextFormat::default()
    };
    let highlighted = TextFormat {
        color: ui.visuals().selection.stroke.color,
        underline: egui::Stroke::new(1.0, ui.visuals().selection.stroke.color),
        ..normal.clone()
    };
    let mut job = egui::text::LayoutJob::default();
    let mut previous = false;
    let mut append = |text: &str, matched: bool| {
        let start = egui::text::ByteIndex(job.text.len());
        job.text.push_str(text);
        let end = egui::text::ByteIndex(job.text.len());
        match job.sections.last_mut() {
            Some(section) if matched == previous => section.byte_range.end = end,
            _ => job.sections.push(egui::text::LayoutSection {
                leading_space: 0.0,
                byte_range: start..end,
                format: if matched { &highlighted } else { &normal }.clone(),
            }),
        }
        previous = matched;
    };
    if start > 0 {
        append("…", false);
    }
    let mut graphemes = text.graphemes(true).enumerate().skip(start);
    for (index, grapheme) in graphemes.by_ref().take(limit) {
        let matched = u32::try_from(index).is_ok_and(|index| indices.binary_search(&index).is_ok());
        append(
            if grapheme.contains(['\r', '\n']) {
                " "
            } else {
                grapheme
            },
            matched,
        );
    }
    if graphemes.next().is_some() {
        append("…", false);
    }
    job
}
