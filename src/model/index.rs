use std::cell::OnceCell;

use chrono::Datelike as _;
use rustc_hash::FxHashMap;

use super::{CategoryId, DateRange, DisplayTimeZone, Document, Event, EventTimeZone, Year};
use intervals::Intervals;
use schedule::DisplayedSchedule;

mod intervals;
pub(super) mod schedule;

#[cfg(test)]
mod timezone_tests;

/// Производная проекция событий: переиспользует буферы и не меняет документ.
#[derive(Default)]
pub struct EventIndex {
    pub months: [MonthEvents; 12],
    rows: Vec<EventOccurrence>,
    intervals: Intervals,
    list_range: Option<DateRange>,
    matching_rows: Vec<OccurrencePosition>,
    projection: Option<Projection>,
    categories: FxHashMap<CategoryId, CategoryStyle>,
}

/// Параметры построенного кэша появляются и сбрасываются вместе.
struct Projection {
    year: Year,
    timezone: DisplayTimeZone,
    range: DateRange,
}

#[derive(Clone, Copy)]
struct CategoryStyle {
    color: [u8; 3],
    visible: bool,
}

impl EventIndex {
    pub const fn invalidate(&mut self) {
        self.projection = None;
    }

    pub fn refresh(&mut self, document: &Document) {
        self.refresh_year(document, document.year);
    }

    pub fn refresh_year(&mut self, document: &Document, year: Year) {
        if self.projection.as_ref().is_none_or(|projection| {
            projection.year != year || projection.timezone != document.display_timezone
        }) {
            self.project(document, year.range(), year);
        }
    }

    pub fn list<'a>(
        &'a mut self,
        document: &'a Document,
        range: DateRange,
    ) -> impl ExactSizeIterator<Item = EventListEntry<'a>> {
        self.refresh(document);
        if !self.projection.as_ref().is_some_and(|projection| {
            projection.range.start() <= range.start() && projection.range.end() >= range.end()
        }) {
            let year = document.year.range();
            self.project(
                document,
                DateRange::between(year.start().min(range.start()), year.end().max(range.end())),
                document.year,
            );
        }
        if self.list_range != Some(range) {
            self.matching_rows.clear();
            self.intervals.for_each_matching(range, |index| {
                self.matching_rows.push(OccurrencePosition(index));
            });
            self.list_range = Some(range);
        }
        self.matching_rows.iter().map(|&OccurrencePosition(index)| {
            let occurrence = &self.rows[index];
            EventListEntry {
                event: occurrence.source.event(document),
                occurrence,
            }
        })
    }

    fn project(&mut self, document: &Document, projection: DateRange, display_year: Year) {
        self.categories.clear();
        self.categories
            .extend(document.groups.iter().flat_map(|group| {
                group.categories.iter().map(move |category| {
                    (
                        category.id,
                        CategoryStyle {
                            color: category.color,
                            visible: group.enabled && category.enabled,
                        },
                    )
                })
            }));
        self.rows.clear();
        let query = schedule::source_query(projection);
        for (event_index, event) in document.events.iter().enumerate() {
            let &CategoryStyle { color, visible } = self
                .categories
                .get(&event.category)
                .expect("Known category");
            self.rows.extend(
                event
                    .occurrences(if event.schedule.timezone() == EventTimeZone::Floating {
                        projection
                    } else {
                        query
                    })
                    .filter_map(|schedule| {
                        DisplayedSchedule::project(schedule, document.display_timezone)
                    })
                    .filter(|schedule| schedule.occupied_dates().overlaps(projection))
                    .map(|schedule| EventOccurrence {
                        source: EventPosition(event_index),
                        schedule,
                        color,
                        visible,
                        date_label: OnceCell::new(),
                    }),
            );
        }
        self.intervals
            .rebuild(self.rows.iter().map(|row| row.schedule.occupied_dates()));
        self.list_range = None;
        for month in &mut self.months {
            month.events.clear();
            month.days.fill(DayMarkers::default());
        }
        let year = display_year.range();
        self.intervals.for_each_matching(year, |index| {
            let row = &self.rows[index];
            if !row.visible {
                return;
            }
            let color = row.color;
            let range = row.schedule.occupied_dates();
            let start = range.start().max(year.start());
            let end = range.end().min(year.end());
            let first_month = usize::try_from(start.month0()).expect("Twelve months");
            let last_month = usize::try_from(end.month0()).expect("Twelve months");
            // Границы вычисляются один раз на месяц; внутренний цикл работает только со срезом дней.
            for (number, month) in
                (start.month()..=end.month()).zip(&mut self.months[first_month..=last_month])
            {
                month.events.push(IndexedEvent {
                    source: row.source,
                    occurrence: OccurrencePosition(index),
                    range,
                    color,
                });
                let first_day = if number == start.month() {
                    start.day0()
                } else {
                    0
                };
                let last_day = if number == end.month() {
                    end.day()
                } else {
                    u32::from(
                        year.start()
                            .with_month(number)
                            .expect("Valid month")
                            .num_days_in_month(),
                    )
                };
                let days = usize::try_from(first_day).expect("At most 31 days")
                    ..usize::try_from(last_day).expect("At most 31 days");
                for day in &mut month.days[days] {
                    day.push(color);
                }
            }
        });
        self.projection = Some(Projection {
            year: display_year,
            timezone: document.display_timezone,
            range: projection,
        });
    }
}

/// Позиция исходного события в документе, действительная до следующего изменения документа.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct EventPosition(usize);

impl EventPosition {
    fn event(self, document: &Document) -> &Event {
        &document.events[self.0]
    }
}

/// Позиция экземпляра повтора в проекции индекса, а не исходного события в документе.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct OccurrencePosition(usize);

/// Исходное событие и конкретный экземпляр его расписания без копирования данных документа.
pub struct EventListEntry<'a> {
    pub event: &'a Event,
    pub occurrence: &'a EventOccurrence,
}

pub struct EventOccurrence {
    source: EventPosition,
    pub color: [u8; 3],
    pub visible: bool,
    schedule: DisplayedSchedule,
    date_label: OnceCell<String>,
}

impl EventOccurrence {
    pub const fn start(&self) -> chrono::NaiveDate {
        self.schedule.start_date()
    }

    pub fn date_label(&self) -> &str {
        self.date_label.get_or_init(|| self.schedule.to_string())
    }
}

#[derive(Default)]
pub struct MonthEvents {
    pub events: Vec<IndexedEvent>,
    pub days: [DayMarkers; 31],
}

pub struct IndexedEvent {
    source: EventPosition,
    occurrence: OccurrencePosition,
    pub range: DateRange,
    pub color: [u8; 3],
}

impl IndexedEvent {
    pub fn date_label<'a>(&self, index: &'a EventIndex) -> &'a str {
        index.rows[self.occurrence.0].date_label()
    }

    pub fn source_event<'a>(&self, document: &'a Document) -> &'a Event {
        self.source.event(document)
    }
}

/// Под датой помещается не больше шести точек; остальные события доступны в подсказке.
#[derive(Clone, Copy, Default)]
pub struct DayMarkers {
    colors: [[u8; 3]; 6],
    len: u8,
}

impl DayMarkers {
    pub fn colors(&self) -> &[[u8; 3]] {
        &self.colors[..usize::from(self.len)]
    }

    fn push(&mut self, color: [u8; 3]) {
        if let Some(slot) = self.colors.get_mut(usize::from(self.len)) {
            *slot = color;
            self.len += 1;
        }
    }
}

#[cfg(test)]
mod tests;
