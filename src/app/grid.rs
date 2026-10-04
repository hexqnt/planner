use std::sync::Arc;

use chrono::{Datelike as _, NaiveDate};
use egui::{Align2, Color32, FontId, Galley, Rect, Sense, Stroke, Vec2};

use config::{DAY_RADIUS, MONTH_GAP};

use crate::{
    calendar::Calendar,
    calendar::{Day, Month},
    model::{CalendarViewMode, DateRange, Document, EventIndex},
};

use super::vacation::Prices;
use super::{BLUE, Planner, RED, selection::Selection, ui_config::calendar as config};

mod continuous;

#[cfg(test)]
mod painting_tests;

pub(super) use continuous::ContinuousView;

pub(super) use config::{DAY_CELL, DAYS_TOP, GRID_MARGIN};

/// Горизонтальный интервал между центрами точек событий.
const MARKER_STEP: f32 = 3.0;

/// Смещение первой точки событий влево от центра дня.
const MARKER_LEFT: f32 = 7.0;

/// Отступ точек событий вверх от нижнего края круга дня.
const MARKER_BOTTOM_INSET: f32 = 2.0;

/// Радиус точек событий.
const MARKER_RADIUS: f32 = 1.5;

#[cfg(test)]
#[allow(clippy::suboptimal_flops)] // mul_add недоступен в const-контексте.
const MONTH_SIZE: Vec2 = Vec2::new(DAY_CELL.x * 7.0, DAYS_TOP + DAY_CELL.y * 6.0);

#[derive(Clone, Copy)]
struct Geometry {
    cell: Vec2,
    month_size: Vec2,
}

impl Geometry {
    fn new(costs: bool) -> Self {
        let cell = if costs {
            config::VACATION_DAY_CELL
        } else {
            DAY_CELL
        };
        Self {
            cell,
            month_size: Vec2::new(cell.x * 7.0, cell.y.mul_add(6.0, DAYS_TOP)),
        }
    }
}

const DAY_LABELS: [&str; 31] = [
    "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15", "16", "17",
    "18", "19", "20", "21", "22", "23", "24", "25", "26", "27", "28", "29", "30", "31",
];

impl Planner {
    pub(super) fn year_grid(&mut self, ui: &mut egui::Ui) {
        if self.document.view_mode == CalendarViewMode::Continuous {
            self.continuous_grid(ui);
            return;
        }
        self.scroll_single_year(ui);
        self.calendar
            .refresh(self.document.year, self.document.region);
        self.event_index.refresh(&self.document);
        let geometry = Geometry::new(self.document.vacation.enabled);
        let pay = self.vacation.pay.filter(|_| self.document.vacation.enabled);
        Prices::refresh(&mut self.vacation.prices, ui, &self.calendar, pay);
        let today = self.document.display_timezone.today();
        // Подписи общие для всех месяцев; цвет задаётся при рисовании, а кэшом шрифтов управляет egui.
        let day_labels = layout_labels(ui, DAY_LABELS, config::DAY_FONT_SIZE);
        let weekday_labels =
            layout_labels(ui, self.language().weekdays(), config::WEEKDAY_FONT_SIZE);
        let columns = columns(ui.available_width(), geometry);
        let mut create_event = false;
        let size = grid_size(columns, geometry);
        let available = ui.available_size();
        // Размер сетки известен заранее; округление координат в egui не должно включать лишнюю прокрутку.
        egui::ScrollArea::new([size.x > available.x, size.y > available.y])
            .auto_shrink([false, false])
            .show_viewport(ui, |ui, viewport| {
                let available = Rect::from_min_size(ui.max_rect().min, viewport.size());
                ui.scope_builder(
                    egui::UiBuilder::new()
                        .max_rect(centered_grid_rect(available, columns, geometry)),
                    |ui| {
                        egui::Grid::new("months")
                            .num_columns(columns)
                            .spacing(Vec2::splat(MONTH_GAP))
                            .show(ui, |ui| {
                                for index in 0..12 {
                                    create_event |=
                                        self.month(ui, index, today, &day_labels, &weekday_labels);
                                    if (index + 1) % columns == 0 {
                                        ui.end_row();
                                    }
                                }
                            });
                    },
                );
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

    fn scroll_single_year(&mut self, ui: &egui::Ui) {
        if self.document.vacation.enabled
            || !ui.is_enabled()
            || !ui.rect_contains_pointer(ui.available_rect_before_wrap())
        {
            return;
        }
        let scroll = ui.input_mut(|input| {
            // Сглаженный хвост колёсика не должен прокручивать сетку после смены года.
            input.smooth_scroll_delta.y = 0.0;
            input
                .events
                .iter()
                .filter_map(|event| match event {
                    egui::Event::MouseWheel {
                        delta, modifiers, ..
                    } if !modifiers.shift && !modifiers.ctrl && !modifiers.command => Some(delta.y),
                    _ => None,
                })
                .sum::<f32>()
        });
        if scroll != 0.0
            && let Some(year) = self.document.year.step(if scroll < 0.0 { 1 } else { -1 })
        {
            self.change_year(year);
            ui.ctx().request_repaint();
        }
    }

    fn month(
        &mut self,
        ui: &mut egui::Ui,
        index: usize,
        today: NaiveDate,
        day_labels: &[Arc<Galley>; 31],
        weekday_labels: &[Arc<Galley>; 7],
    ) -> bool {
        MonthView {
            document: &self.document,
            calendar: &self.calendar,
            events: &self.event_index,
            selection: &mut self.selection,
            today,
            day_labels,
            weekday_labels,
            geometry: Geometry::new(self.document.vacation.enabled),
            prices: self.vacation.prices.as_ref(),
        }
        .month(ui, index)
    }
}

struct MonthView<'a> {
    document: &'a Document,
    calendar: &'a Calendar,
    events: &'a EventIndex,
    selection: &'a mut Selection,
    today: NaiveDate,
    day_labels: &'a [Arc<Galley>; 31],
    weekday_labels: &'a [Arc<Galley>; 7],
    geometry: Geometry,
    prices: Option<&'a Prices>,
}

impl MonthView<'_> {
    fn month(&mut self, ui: &mut egui::Ui, index: usize) -> bool {
        let (rect, _) = ui.allocate_exact_size(self.geometry.month_size, Sense::hover());
        if !ui.is_rect_visible(rect) {
            return false;
        }
        let language = self.document.language;
        let month = &self.calendar.months[index];
        let origin = month_heading(
            ui,
            rect,
            language.months()[index],
            self.weekday_labels,
            self.document.dark,
            self.geometry,
        );
        let events = &self.events.months[index];
        for event in &events.events {
            paint_range(
                ui,
                month,
                event.range,
                origin,
                event_color(event.color, self.document.dark),
                None,
                self.geometry,
            );
        }
        if let Some(range) = self.selection.range() {
            paint_range(
                ui,
                month,
                range,
                origin,
                BLUE.gamma_multiply(config::SELECTION_OPACITY),
                Some(Stroke::new(config::SELECTION_STROKE, BLUE)),
                self.geometry,
            );
        }
        let (shift, drag_position) = ui.input(|input| {
            let pointer = &input.pointer;
            let drag_position = if pointer.primary_down() || pointer.primary_released() {
                pointer.hover_pos()
            } else {
                None
            };
            (input.modifiers.shift, drag_position)
        });
        let mut create_event = false;
        for (day_index, ((day, markers), label)) in month
            .days
            .iter()
            .copied()
            .zip(&events.days)
            .zip(self.day_labels)
            .enumerate()
        {
            let rect = day_rect(month, day_index, origin, self.geometry);
            if !ui.is_rect_visible(rect) {
                continue;
            }
            let response = ui.interact(rect, ui.id().with(day.date), Sense::click_and_drag());
            response.widget_info(|| {
                egui::WidgetInfo::labeled(
                    egui::WidgetType::Button,
                    response.enabled(),
                    day.date.to_string(),
                )
            });
            paint_day(
                ui,
                day,
                label,
                &response,
                markers.colors(),
                self.selection.range(),
                DayPresentation {
                    today: self.today,
                    price: self.prices.map(|prices| prices.label(index, day)),
                },
            );
            select_day(
                ui,
                day.date,
                &response,
                self.selection,
                shift,
                drag_position,
            );
            create_event |= response.double_clicked();
            if response.hovered() && !ui.input(|input| input.pointer.primary_down()) {
                let mut response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
                // Общий ID сохраняет хинт при переходе между датами; обработка кликов выше использует ID дня.
                response.id = ui.id().with("day_tooltip");
                egui::Tooltip::for_enabled(&response).show(|ui| {
                    self.tooltip(ui, day, index);
                });
            }
        }
        create_event
    }

    fn tooltip(&self, ui: &mut egui::Ui, day: Day, index: usize) {
        let language = self.document.language;
        day_tooltip(ui, day, language);
        if let Some(prices) = self.prices {
            ui.label(format!(
                "{}: {} ₽",
                language.text("Изменение дохода за день", "Daily income change"),
                prices.label(index, day).text()
            ));
        }
        for event in &self.events.months[index].events {
            if event.range.contains(day.date) {
                ui.horizontal(|ui| {
                    ui.label(super::widgets::calendar_marker(event.color));
                    ui.label(event.source_event(self.document).title.get());
                });
                ui.weak(event.date_label(self.events));
            }
        }
        ui.weak(language.text(
            "Выберите день · Shift + клик или перетаскивание — интервал",
            "Choose a day · Shift + click or drag for a range",
        ));
        ui.weak(language.text(
            "Двойной клик — новое событие",
            "Double-click to add an event",
        ));
    }
}

fn event_color([r, g, b]: [u8; 3], dark: bool) -> Color32 {
    Color32::from_rgb(r, g, b).gamma_multiply(if dark {
        config::EVENT_DARK_OPACITY
    } else {
        config::EVENT_LIGHT_OPACITY
    })
}

fn layout_labels<const N: usize>(ui: &egui::Ui, labels: [&str; N], size: f32) -> [Arc<Galley>; N] {
    ui.fonts_mut(|fonts| {
        labels.map(|label| {
            fonts.layout_no_wrap(
                label.to_owned(),
                FontId::proportional(size),
                Color32::PLACEHOLDER,
            )
        })
    })
}

fn columns(width: f32, geometry: Geometry) -> usize {
    (2..=6)
        .rev()
        .find(|&count| grid_size(count, geometry).x <= width)
        .unwrap_or(2)
}

fn grid_size(columns: usize, geometry: Geometry) -> Vec2 {
    let columns = u16::try_from(columns).expect("At most six columns");
    let rows = f32::from(12_u16.div_ceil(columns));
    Vec2::new(
        f32::from(columns).mul_add(geometry.month_size.x + MONTH_GAP, -MONTH_GAP),
        rows.mul_add(geometry.month_size.y + MONTH_GAP, -MONTH_GAP),
    )
}

fn centered_grid_rect(available: Rect, columns: usize, geometry: Geometry) -> Rect {
    let size = grid_size(columns, geometry);
    let padding = (available.size() - size).max(Vec2::ZERO) / 2.0;
    Rect::from_min_size(available.min + padding, size)
}

fn month_heading(
    ui: &egui::Ui,
    rect: Rect,
    name: &str,
    weekday_labels: &[Arc<Galley>; 7],
    dark: bool,
    geometry: Geometry,
) -> egui::Pos2 {
    let painter = ui.painter();
    let normal = ui.visuals().text_color();
    let muted = if dark {
        config::DARK_MUTED
    } else {
        config::LIGHT_MUTED
    };
    painter.text(
        rect.min + config::MONTH_TITLE_OFFSET,
        Align2::LEFT_TOP,
        name,
        egui::TextStyle::Heading.resolve(ui.style()),
        normal,
    );
    let origin = rect.min + Vec2::new(0.0, DAYS_TOP);
    for (column, label) in weekday_labels.iter().enumerate() {
        let column = f32::from(u16::try_from(column).expect("Seven weekdays"));
        let center = egui::pos2(
            (column + 0.5).mul_add(geometry.cell.x, rect.left()),
            rect.top() + config::WEEKDAY_TOP,
        );
        painter.galley(
            Align2::CENTER_CENTER.anchor_size(center, label.size()).min,
            Arc::clone(label),
            if column >= 5.0 { RED } else { muted },
        );
    }
    origin
}

#[derive(Clone, Copy)]
struct DayPresentation<'a> {
    today: NaiveDate,
    price: Option<&'a Arc<Galley>>,
}

fn paint_day(
    ui: &egui::Ui,
    day: Day,
    label: &Arc<Galley>,
    response: &egui::Response,
    markers: &[[u8; 3]],
    selection: Option<DateRange>,
    presentation: DayPresentation<'_>,
) {
    let center = day_center(response.rect);
    let radius = f32::from(DAY_RADIUS);
    let endpoint =
        selection.is_some_and(|range| day.date == range.start() || day.date == range.end());
    if endpoint {
        ui.painter().circle_filled(center, radius, BLUE);
    } else if response.hovered() {
        ui.painter()
            .circle_filled(center, radius, ui.visuals().widgets.hovered.weak_bg_fill);
    }
    if day.date == presentation.today && !endpoint {
        ui.painter()
            .circle_stroke(center, radius, Stroke::new(config::TODAY_STROKE, BLUE));
    }
    let color = if endpoint {
        super::ui_config::style::TEXT_ON_ACCENT
    } else if day.flags.is_day_off() {
        RED
    } else {
        ui.visuals().text_color()
    };
    ui.painter().galley(
        Align2::CENTER_CENTER.anchor_size(center, label.size()).min,
        Arc::clone(label),
        color,
    );
    for (index, &[r, g, b]) in markers.iter().enumerate() {
        let x = f32::from(u16::try_from(index).expect("At most six markers"))
            .mul_add(MARKER_STEP, center.x - MARKER_LEFT);
        ui.painter().circle_filled(
            egui::pos2(x, center.y + radius - MARKER_BOTTOM_INSET),
            MARKER_RADIUS,
            Color32::from_rgb(r, g, b),
        );
    }
    if let Some(price) = presentation.price {
        let position = egui::pos2(center.x, response.rect.bottom() - 11.0);
        ui.painter().galley(
            Align2::CENTER_CENTER
                .anchor_size(position, price.size())
                .min,
            Arc::clone(price),
            ui.visuals().weak_text_color(),
        );
    }
}

fn select_day(
    ui: &egui::Ui,
    date: NaiveDate,
    response: &egui::Response,
    selection: &mut Selection,
    shift: bool,
    drag_position: Option<egui::Pos2>,
) {
    let before = *selection;
    if response.clicked() {
        selection.click(date, shift);
    }
    if response.drag_started() {
        selection.start_drag(date);
    }
    if selection.is_dragging() && drag_position.is_some_and(|pos| response.rect.contains(pos)) {
        selection.drag_to(date);
    }
    if *selection != before {
        ui.ctx().request_repaint();
    }
}

fn day_rect(month: &Month, index: usize, origin: egui::Pos2, geometry: Geometry) -> Rect {
    let slot = month.offset + u32::try_from(index).expect("At most 31 days");
    let x = f32::from(u16::try_from(slot % 7).expect("Seven columns"));
    let y = f32::from(u16::try_from(slot / 7).expect("Six rows"));
    Rect::from_min_size(
        origin + Vec2::new(x * geometry.cell.x, y * geometry.cell.y),
        geometry.cell,
    )
}

fn day_center(rect: Rect) -> egui::Pos2 {
    rect.center() - Vec2::new(0.0, (rect.height() - DAY_CELL.y) * 0.5)
}

fn paint_range(
    ui: &egui::Ui,
    month: &Month,
    range: DateRange,
    origin: egui::Pos2,
    color: Color32,
    stroke: Option<Stroke>,
    geometry: Geometry,
) {
    let (Some(first_day), Some(last_day)) = (month.days.first(), month.days.last()) else {
        return;
    };
    let start = range.start().max(first_day.date);
    let end = range.end().min(last_day.date);
    if start > end {
        return;
    }
    let first_slot = month.offset + start.day0();
    let last_slot = month.offset + end.day0();
    for row in first_slot / 7..=last_slot / 7 {
        let first =
            usize::try_from(first_slot.max(row * 7) - month.offset).expect("At most 31 days");
        let last =
            usize::try_from(last_slot.min(row * 7 + 6) - month.offset).expect("At most 31 days");
        if first == last && (stroke.is_some() || geometry.cell == DAY_CELL) {
            // Скруглённый квадрат при дробном масштабе может дать перекрывающиеся треугольники в egui.
            let center = day_center(day_rect(month, first, origin, geometry));
            let radius = f32::from(DAY_RADIUS);
            ui.painter().circle_filled(center, radius, color);
            if let Some(stroke) = stroke {
                ui.painter()
                    .circle_stroke(center, radius - stroke.width / 2.0, stroke);
            }
            continue;
        }
        let rect = range_rect(month, first, last, origin, geometry, stroke.is_some());
        ui.painter()
            .rect_filled(rect, egui::CornerRadius::same(DAY_RADIUS), color);
        if let Some(stroke) = stroke {
            ui.painter().rect_stroke(
                rect,
                egui::CornerRadius::same(DAY_RADIUS),
                stroke,
                egui::StrokeKind::Inside,
            );
        }
    }
}

fn range_rect(
    month: &Month,
    first: usize,
    last: usize,
    origin: egui::Pos2,
    geometry: Geometry,
    outlined: bool,
) -> Rect {
    let first = day_rect(month, first, origin, geometry);
    let last = day_rect(month, last, origin, geometry);
    if outlined {
        let radius = Vec2::splat(f32::from(DAY_RADIUS));
        return Rect::from_min_max(day_center(first) - radius, day_center(last) + radius);
    }
    let radius = if geometry.cell == DAY_CELL {
        Vec2::splat(f32::from(DAY_RADIUS))
    } else {
        geometry.cell / 2.0 - Vec2::splat(2.0)
    };
    Rect::from_min_max(first.center() - radius, last.center() + radius)
}

fn day_tooltip(ui: &mut egui::Ui, day: Day, language: crate::text::Language) {
    ui.strong(day.date.format("%d.%m.%Y").to_string());
    let kind = if day.flags.is_holiday() {
        language.text("Праздничный день", "Public holiday")
    } else if day.flags.is_day_off() {
        language.text("Выходной день", "Day off")
    } else if day.flags.is_short_day() {
        language.text("Сокращённый рабочий день", "Short working day")
    } else {
        language.text("Рабочий день", "Working day")
    };
    ui.label(kind);
    if day.flags.is_transferred() {
        ui.weak(language.text("Перенос выходного", "Transferred day"));
    }
    if day.predicted {
        ui.weak(language.text(
            "Прогноз производственного календаря",
            "Predicted production calendar",
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wheel_frame(
        ctx: &egui::Context,
        planner: &mut Planner,
        pointer: egui::Pos2,
        delta: Vec2,
        modifiers: egui::Modifiers,
        enabled: bool,
    ) {
        ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(
                    egui::Pos2::ZERO,
                    Vec2::new(500.0, 300.0),
                )),
                events: vec![
                    egui::Event::PointerMoved(pointer),
                    egui::Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Line,
                        phase: egui::TouchPhase::Move,
                        delta,
                        modifiers,
                    },
                ],
                ..Default::default()
            },
            |ui| {
                ui.add_enabled_ui(enabled, |ui| {
                    let id = ui.make_persistent_id(egui::IdSalt::new("scroll_area"));
                    planner.year_grid(ui);
                    if enabled && modifiers == egui::Modifiers::NONE && delta.y != 0.0 {
                        assert_eq!(
                            egui::scroll_area::State::load(ctx, id).unwrap().offset.y,
                            0.0
                        );
                    }
                });
            },
        )
        .drop_without_applying_deltas();
    }

    #[test]
    fn single_year_wheel_changes_year_without_scrolling_the_grid_or_repeating_on_idle() {
        let ctx = egui::Context::default();
        let document = Document {
            year: crate::model::Year::try_from(2026).unwrap(),
            view_mode: CalendarViewMode::SingleYear,
            ..Document::default()
        };
        let mut planner = Planner::from_document(document, &ctx, None);
        let pointer = egui::pos2(250.0, 150.0);
        for (delta, expected) in [(-1.0, 2027), (-1.0, 2028), (1.0, 2027)] {
            wheel_frame(
                &ctx,
                &mut planner,
                pointer,
                Vec2::new(0.0, delta),
                egui::Modifiers::NONE,
                true,
            );
            assert_eq!(planner.document.year.get(), expected);
            assert_eq!(planner.calendar.year, planner.document.year);
            for _ in 0..10 {
                ctx.run_ui(egui::RawInput::default(), |ui| {
                    ui.set_max_size(Vec2::new(500.0, 300.0));
                    ui.add_enabled_ui(true, |ui| {
                        let id = ui.make_persistent_id(egui::IdSalt::new("scroll_area"));
                        planner.year_grid(ui);
                        assert_eq!(
                            egui::scroll_area::State::load(&ctx, id).unwrap().offset.y,
                            0.0
                        );
                    });
                })
                .drop_without_applying_deltas();
                assert_eq!(planner.document.year.get(), expected);
            }
        }
    }

    #[test]
    fn single_year_wheel_respects_hover_modifiers_enabled_state_and_year_bounds() {
        let ctx = egui::Context::default();
        let document = Document {
            year: crate::model::Year::try_from(2026).unwrap(),
            view_mode: CalendarViewMode::SingleYear,
            ..Document::default()
        };
        let mut planner = Planner::from_document(document, &ctx, None);
        let inside = egui::pos2(250.0, 150.0);
        let vertical = Vec2::new(0.0, -1.0);
        for (pointer, delta, modifiers, enabled) in [
            (
                egui::pos2(600.0, 150.0),
                vertical,
                egui::Modifiers::NONE,
                true,
            ),
            (inside, vertical, egui::Modifiers::SHIFT, true),
            (inside, vertical, egui::Modifiers::CTRL, true),
            (inside, vertical, egui::Modifiers::COMMAND, true),
            (inside, Vec2::new(-1.0, 0.0), egui::Modifiers::NONE, true),
            (inside, vertical, egui::Modifiers::NONE, false),
        ] {
            wheel_frame(&ctx, &mut planner, pointer, delta, modifiers, enabled);
            assert_eq!(planner.document.year.get(), 2026);
        }
        for (year, delta) in [(holidays_ru::MIN_YEAR, 1.0), (holidays_ru::MAX_YEAR, -1.0)] {
            planner.change_year(crate::model::Year::try_from(year).unwrap());
            wheel_frame(
                &ctx,
                &mut planner,
                inside,
                Vec2::new(0.0, delta),
                egui::Modifiers::NONE,
                true,
            );
            assert_eq!(planner.document.year.get(), year);
        }
    }

    #[test]
    fn day_tooltip_stays_visible_when_moving_between_dates() {
        let ctx = egui::Context::default();
        let document = crate::model::Document {
            year: crate::model::Year::try_from(2026).unwrap(),
            ..crate::model::Document::default()
        };
        let mut planner = Planner::from_document(document, &ctx, None);
        let mut render = |time, pointer: Option<egui::Pos2>| {
            ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::splat(2000.0))),
                    time: Some(time),
                    events: pointer.map(egui::Event::PointerMoved).into_iter().collect(),
                    ..Default::default()
                },
                |ui| planner.year_grid(ui),
            )
        };
        let output = render(0.0, None);
        let day_center = |label| {
            output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) if text.galley.text() == label => {
                        Some(text.pos + text.galley.size() / 2.0)
                    }
                    _ => None,
                })
                .unwrap()
        };
        let first = day_center("1");
        let second = day_center("2");
        output.drop_without_applying_deltas();
        render(0.1, Some(first)).drop_without_applying_deltas();
        for time in [1.0, 1.1] {
            render(time, None).drop_without_applying_deltas();
        }
        for (time, pointer, date) in [
            (1.2, first, "01.01.2026"),
            (1.21, second, "02.01.2026"),
            (1.22, first, "01.01.2026"),
        ] {
            let output = render(time, Some(pointer));
            assert!(
                output.shapes.iter().any(|shape| {
                    matches!(&shape.shape, egui::Shape::Text(text) if text.galley.text() == date)
                }),
                "Tooltip for {date} disappeared while moving between dates"
            );
            output.drop_without_applying_deltas();
        }
        let output = render(1.3, Some(egui::Pos2::ZERO));
        assert!(!output.shapes.iter().any(|shape| {
            matches!(&shape.shape, egui::Shape::Text(text) if text.galley.text() == "01.01.2026")
        }));
        output.drop_without_applying_deltas();
    }

    #[test]
    fn day_labels_share_layout_across_months_and_keep_individual_colors() {
        let ctx = egui::Context::default();
        let document = crate::model::Document {
            year: crate::model::Year::try_from(2026).unwrap(),
            ..crate::model::Document::default()
        };
        let mut planner = Planner::from_document(document, &ctx, None);
        let selected = "2026-03-01".parse().unwrap();
        planner
            .selection
            .set_range(DateRange::between(selected, selected));
        let output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::splat(2000.0))),
                ..Default::default()
            },
            |ui| planner.year_grid(ui),
        );
        let labels: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.text() == "1" => Some(text),
                _ => None,
            })
            .collect();
        assert_eq!(labels.len(), 12);
        for label in &labels {
            assert!(Arc::ptr_eq(&labels[0].galley, &label.galley));
        }
        assert_eq!(labels[0].fallback_color, RED);
        assert_eq!(labels[2].fallback_color, Color32::WHITE);
        assert_ne!(labels[3].fallback_color, RED);
        assert_ne!(labels[3].fallback_color, Color32::WHITE);
        output.drop_without_applying_deltas();
    }

    #[test]
    fn weekday_labels_follow_language_and_theme_changes() {
        let ctx = egui::Context::default();
        let mut planner = Planner::from_document(crate::model::Document::default(), &ctx, None);
        for language in [
            crate::text::Language::Russian,
            crate::text::Language::English,
        ] {
            for dark in [false, true] {
                planner.document.language = language;
                planner.document.dark = dark;
                let output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(
                            egui::Pos2::ZERO,
                            Vec2::splat(2000.0),
                        )),
                        ..Default::default()
                    },
                    |ui| planner.year_grid(ui),
                );
                for (column, name) in language.weekdays().into_iter().enumerate() {
                    let labels: Vec<_> = output
                        .shapes
                        .iter()
                        .filter_map(|shape| match &shape.shape {
                            egui::Shape::Text(text) if text.galley.text() == name => Some(text),
                            _ => None,
                        })
                        .collect();
                    assert_eq!(labels.len(), 12);
                    let color = if column >= 5 {
                        RED
                    } else if dark {
                        Color32::from_rgb(127, 145, 174)
                    } else {
                        Color32::from_rgb(120, 137, 166)
                    };
                    for label in &labels {
                        assert!(Arc::ptr_eq(&labels[0].galley, &label.galley));
                        assert_eq!(label.fallback_color, color);
                    }
                }
                output.drop_without_applying_deltas();
            }
        }
    }

    #[test]
    fn clipped_days_are_not_painted() {
        let ctx = egui::Context::default();
        let document = crate::model::Document {
            year: crate::model::Year::try_from(2026).unwrap(),
            ..crate::model::Document::default()
        };
        let mut planner = Planner::from_document(document, &ctx, None);
        let output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(
                    egui::Pos2::ZERO,
                    Vec2::new(500.0, 150.0),
                )),
                ..Default::default()
            },
            |ui| planner.year_grid(ui),
        );
        let mut count = 0;
        for shape in &output.shapes {
            if let egui::Shape::Text(text) = &shape.shape
                && text.galley.text().parse::<u32>().is_ok()
            {
                count += 1;
                let center = text.pos + text.galley.size() / 2.0;
                let cell = Rect::from_center_size(center, DAY_CELL);
                assert!(shape.clip_rect.intersects(cell));
            }
        }
        assert!(count > 0);
        assert!(count < 31 + 28);
        output.drop_without_applying_deltas();
    }

    #[test]
    fn range_painting_covers_only_included_days_across_rows_and_months() {
        let calendar = crate::calendar::Calendar::new(
            crate::model::Year::try_from(2024).unwrap(),
            crate::calendar::Region::Federal,
        );
        for (start, end) in [
            ("2023-12-28", "2024-03-01"),
            ("2024-02-12", "2024-02-20"),
            ("2024-02-29", "2024-02-29"),
            ("2024-12-28", "2025-01-08"),
        ] {
            let range = DateRange::new(start.parse().unwrap(), end.parse().unwrap()).unwrap();
            for month in &calendar.months {
                let ctx = egui::Context::default();
                let origin = egui::pos2(10.0, 10.0);
                let output = ctx.run_ui(egui::RawInput::default(), |ui| {
                    paint_range(ui, month, range, origin, BLUE, None, Geometry::new(false));
                });
                for (index, day) in month.days.iter().enumerate() {
                    let center = day_rect(month, index, origin, Geometry::new(false)).center();
                    let covered = output
                        .shapes
                        .iter()
                        .any(|shape| shape.shape.visual_bounding_rect().contains(center));
                    assert_eq!(covered, range.contains(day.date), "{}", day.date);
                }
                output.drop_without_applying_deltas();
            }
        }
    }

    #[test]
    fn scrollbars_follow_grid_overflow_at_fractional_viewport_sizes() {
        let ctx = egui::Context::default();
        let mut planner = Planner::from_document(Document::default(), &ctx, None);
        for count in (2..=6).rev() {
            let size = grid_size(count, Geometry::new(false));
            for delta in [-20.0, 0.0, 0.25, 0.5, 1.0, 20.0] {
                let screen = Vec2::new(size.x + 0.5, size.y + delta);
                for _ in 0..3 {
                    ctx.run_ui(
                        egui::RawInput {
                            screen_rect: Some(Rect::from_min_size(egui::pos2(0.25, 0.25), screen)),
                            ..Default::default()
                        },
                        |ui| {
                            let id = ui.make_persistent_id(egui::IdSalt::new("scroll_area"));
                            planner.year_grid(ui);
                            let state = egui::scroll_area::State::load(&ctx, id).unwrap();
                            let state = serde_json::to_value(state).unwrap();
                            assert_eq!(
                                state["show_scroll"],
                                serde_json::json!({"x": false, "y": delta < 0.0})
                            );
                        },
                    )
                    .drop_without_applying_deltas();
                }
            }
        }
    }

    #[test]
    fn grid_is_centered_only_on_axes_that_fit() {
        let large = Rect::from_min_size(egui::pos2(250.0, 72.0), Vec2::new(1500.0, 1200.0));
        let centered = centered_grid_rect(large, 4, Geometry::new(false));
        assert_eq!(centered.center(), large.center());
        let tall = centered_grid_rect(large, 2, Geometry::new(false));
        assert!((tall.top() - large.top()).abs() < f32::EPSILON);
        assert!((tall.center().x - large.center().x).abs() < f32::EPSILON);
        let small = Rect::from_min_size(large.min, Vec2::new(400.0, 300.0));
        assert_eq!(
            centered_grid_rect(small, 2, Geometry::new(false)).min,
            small.min
        );
    }

    #[test]
    fn wide_layouts_fit_and_include_the_partial_fifth_column_row() {
        for (width, expected) in [
            (450.0, 2),
            (720.0, 3),
            (968.0, 4),
            (1215.0, 4),
            (1216.0, 5),
            (1463.0, 5),
            (1464.0, 6),
            (3000.0, 6),
        ] {
            assert_eq!(columns(width, Geometry::new(false)), expected);
        }
        let available = Rect::from_min_size(egui::pos2(250.0, 72.0), Vec2::new(1600.0, 1200.0));
        for count in [5, 6] {
            let grid = centered_grid_rect(available, count, Geometry::new(false));
            assert_eq!(grid.center(), available.center());
            let last_row = (12 - 1) / count;
            let row = f32::from(u16::try_from(last_row).unwrap());
            assert!(
                (grid.height() - row.mul_add(MONTH_SIZE.y + MONTH_GAP, MONTH_SIZE.y)).abs()
                    < f32::EPSILON
            );
        }
    }
}
