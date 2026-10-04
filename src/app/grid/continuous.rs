use chrono::{Datelike as _, NaiveDate};
use egui::{Align2, FontId, Rect, Stroke, Vec2};

use crate::{
    calendar::Calendar,
    model::{Document, EventIndex, Year},
};

use super::{
    DAY_LABELS, Geometry, MONTH_GAP, MonthView, Planner, columns, config, grid_size, layout_labels,
};

use acceleration::WheelAcceleration;
use smooth::SmoothScroll;

mod acceleration;
mod smooth;

const SEPARATOR_HEIGHT: f32 = 72.0;
fn year_count() -> u16 {
    u16::try_from(holidays_ru::MAX_YEAR - holidays_ru::MIN_YEAR + 1).expect("Supported year count")
}

struct CachedYear {
    calendar: Calendar,
    events: EventIndex,
    prices: Option<crate::app::vacation::Prices>,
}

#[derive(Default)]
pub(in crate::app) struct ContinuousView {
    years: Vec<CachedYear>,
    pending: Option<(Year, Option<NaiveDate>)>,
    geometry: Option<(f32, f32)>,
    #[cfg(feature = "testing")]
    layout: Option<(usize, Rect)>,
    wheel: WheelAcceleration,
    smooth: SmoothScroll,
}

impl ContinuousView {
    #[cfg(feature = "testing")]
    pub(in crate::app) fn inspect(&self) -> Option<crate::app::testing::CalendarState> {
        let (stride, offset) = self.geometry?;
        let (columns, rect) = self.layout?;
        Some(crate::app::testing::CalendarState {
            stride,
            offset,
            columns,
            rect,
            cached_years: self.years.len(),
            has_events: self.years.iter().any(|year| {
                year.events
                    .months
                    .iter()
                    .any(|month| !month.events.is_empty())
            }),
        })
    }

    pub(in crate::app) fn invalidate(&mut self) {
        for year in &mut self.years {
            year.events.invalidate();
        }
    }

    fn visible_years(&mut self, viewport: Rect, stride: f32) -> std::ops::RangeInclusive<u16> {
        let first = block_at(viewport.top(), stride).saturating_sub(1);
        let last = (block_at(viewport.bottom(), stride) + 1).min(year_count() - 1);
        self.years.retain(|entry| {
            let year = entry.calendar.year.get();
            year >= year_at(first).get() && year <= year_at(last).get()
        });
        first..=last
    }

    fn year_mut(&mut self, document: &Document, year: Year) -> &mut CachedYear {
        let index = self
            .years
            .iter()
            .position(|entry| entry.calendar.year == year)
            .unwrap_or_else(|| {
                self.years.push(CachedYear {
                    calendar: Calendar::new(year, document.region),
                    events: EventIndex::default(),
                    prices: None,
                });
                self.years.len() - 1
            });
        let entry = &mut self.years[index];
        entry.calendar.refresh(year, document.region);
        entry.events.refresh_year(document, year);
        entry
    }

    fn scroll_area(
        &mut self,
        ui: &egui::Ui,
        year: Year,
        columns: usize,
        stride: f32,
        geometry: Geometry,
    ) -> egui::ScrollArea {
        if self.geometry.is_none() && self.pending.is_none() {
            self.jump_to(year, None);
        }
        let offset = if let Some((year, date)) = self.pending.take() {
            year_offset(year, stride)
                + date.map_or(0.0, |date| {
                    date_offset(date, columns, geometry)
                        - viewport_anchor(ui.available_height(), stride)
                })
        } else if let Some((old_stride, offset)) = self.geometry {
            if (old_stride - stride).abs() > f32::EPSILON {
                self.smooth.cancel();
                offset / old_stride * stride
            } else {
                offset
            }
        } else {
            0.0
        };
        let max_offset = content_height(ui.available_height(), stride) - ui.available_height();
        let multiplier = self.wheel.multiplier(ui);
        let offset = self.smooth.offset(ui, offset, max_offset, multiplier);
        egui::ScrollArea::vertical()
            .id_salt("continuous_vertical")
            .vertical_scroll_offset(offset)
            .scroll_source(egui::scroll_area::ScrollSource {
                mouse_wheel: false,
                ..Default::default()
            })
            .auto_shrink([false, false])
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
    }

    pub(in crate::app) const fn jump_to(&mut self, year: Year, date: Option<NaiveDate>) {
        self.pending = Some((year, date));
        self.wheel = WheelAcceleration::new();
        self.smooth.cancel();
    }
}

fn year_at(index: u16) -> Year {
    Year::clamped(holidays_ru::MIN_YEAR + i32::from(index))
}

fn year_offset(year: Year, stride: f32) -> f32 {
    f32::from(u16::try_from(year.get() - holidays_ru::MIN_YEAR).expect("Supported year")) * stride
}

/// При высоком окне точка выбора года остаётся в пределах первого календаря.
fn viewport_anchor(height: f32, stride: f32) -> f32 {
    height.min(stride - SEPARATOR_HEIGHT) / 2.0
}

fn content_height(height: f32, stride: f32) -> f32 {
    // Запас снизу позволяет поставить последний год в начало высокого окна.
    f32::from(year_count() - 1).mul_add(stride, height.max(stride - SEPARATOR_HEIGHT))
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn block_at(position: f32, stride: f32) -> u16 {
    (position / stride)
        .floor()
        .clamp(0.0, f32::from(year_count() - 1)) as u16
}

fn date_offset(date: NaiveDate, columns: usize, geometry: Geometry) -> f32 {
    let month = u16::try_from(date.month0()).expect("Twelve months");
    let row = month / u16::try_from(columns).expect("At most six columns");
    let first = date.with_day(1).expect("Valid first day");
    let week = (first.weekday().num_days_from_monday() + date.day0()) / 7;
    (f32::from(u16::try_from(week).expect("Six weeks")) + 0.5).mul_add(
        geometry.cell.y,
        f32::from(row).mul_add(geometry.month_size.y + MONTH_GAP, config::DAYS_TOP),
    )
}

impl Planner {
    pub(super) fn continuous_grid(&mut self, ui: &mut egui::Ui) {
        let today = self.document.display_timezone.today();
        let geometry = Geometry::new(self.document.vacation.enabled);
        let pay = self.vacation.pay.filter(|_| self.document.vacation.enabled);
        let day_labels = layout_labels(ui, DAY_LABELS, config::DAY_FONT_SIZE);
        let weekday_labels =
            layout_labels(ui, self.language().weekdays(), config::WEEKDAY_FONT_SIZE);
        let columns = columns(ui.available_width(), geometry);
        #[cfg(feature = "testing")]
        {
            self.continuous.layout = Some((columns, ui.available_rect_before_wrap()));
        }
        let size = grid_size(columns, geometry);
        let stride = size.y + SEPARATOR_HEIGHT;
        let mut create_event = false;
        // Число колонок уменьшается до двух; горизонтальная прокрутка нужна только если и они не помещаются.
        egui::ScrollArea::new([size.x > ui.available_width(), false])
            .id_salt("continuous_horizontal")
            .auto_shrink([false, false])
            .show_viewport(ui, |ui, viewport| {
                // Ширина внутреннего Ui округляется к пикселям и может превысить внешний viewport.
                let scroll = self
                    .continuous
                    .scroll_area(ui, self.document.year, columns, stride, geometry)
                    .max_width(size.x.max(viewport.width()));
                let output = scroll.show_viewport(ui, |ui, viewport| {
                    let width = size.x.max(viewport.width());
                    ui.set_min_size(Vec2::new(width, content_height(viewport.height(), stride)));
                    let origin = ui.max_rect().min + Vec2::new((width - size.x) / 2.0, 0.0);
                    for index in self.continuous.visible_years(viewport, stride) {
                        let year = year_at(index);
                        let entry = self.continuous.year_mut(&self.document, year);
                        crate::app::vacation::Prices::refresh(
                            &mut entry.prices,
                            ui,
                            &entry.calendar,
                            pay,
                        );
                        let top = origin + Vec2::new(0.0, f32::from(index) * stride);
                        let mut view = MonthView {
                            document: &self.document,
                            calendar: &entry.calendar,
                            events: &entry.events,
                            selection: &mut self.selection,
                            today,
                            day_labels: &day_labels,
                            weekday_labels: &weekday_labels,
                            geometry,
                            prices: entry.prices.as_ref(),
                        };
                        for month in 0..12_u16 {
                            let count = u16::try_from(columns).expect("At most six columns");
                            let pos = top
                                + Vec2::new(
                                    f32::from(month % count) * (geometry.month_size.x + MONTH_GAP),
                                    f32::from(month / count) * (geometry.month_size.y + MONTH_GAP),
                                );
                            let rect = Rect::from_min_size(pos, geometry.month_size);
                            if ui.is_rect_visible(rect) {
                                ui.scope_builder(
                                    egui::UiBuilder::new()
                                        .id_salt((year.get(), month))
                                        .max_rect(rect),
                                    |ui| {
                                        create_event |= view.month(ui, usize::from(month));
                                    },
                                );
                            }
                        }
                        if index + 1 < year_count() {
                            let rect = Rect::from_min_size(
                                top + Vec2::new(0.0, size.y),
                                Vec2::new(size.x, SEPARATOR_HEIGHT),
                            );
                            if ui.is_rect_visible(rect) {
                                year_separator(ui, rect, year);
                            }
                        }
                    }
                    let active = year_at(block_at(
                        viewport.top()
                            + viewport_anchor(viewport.height(), stride)
                            + SEPARATOR_HEIGHT / 2.0,
                        stride,
                    ));
                    if self.document.year != active {
                        self.document.year = active;
                        self.persistence.mark_changed();
                        self.event_index.invalidate();
                        ui.ctx().request_repaint();
                    }
                });
                self.continuous.geometry = Some((stride, output.state.offset.y));
            });
        if !ui.input(|input| input.pointer.primary_down()) {
            if self.selection.is_dragging() {
                ui.ctx().request_repaint();
            }
            self.selection.finish_drag();
        }
        if create_event {
            self.new_event();
        }
    }
}

fn year_separator(ui: &egui::Ui, rect: Rect, year: Year) {
    let y = rect.center().y;
    let color = ui.visuals().widgets.noninteractive.bg_stroke.color;
    ui.painter()
        .hline(rect.x_range(), y, Stroke::new(1.0, color));
    for (year, anchor, y) in [
        (year, Align2::RIGHT_BOTTOM, y - 4.0),
        (
            year.step(1).expect("Separator before the last year"),
            Align2::RIGHT_TOP,
            y + 4.0,
        ),
    ] {
        ui.painter().text(
            egui::pos2(rect.right(), y),
            anchor,
            year.get().to_string(),
            FontId::proportional(15.0),
            ui.visuals().weak_text_color(),
        );
    }
}

#[cfg(test)]
mod tests;
