//! Поиск по исходным событиям; текст индексируется отдельно от экземпляров повторений.

use std::cell::OnceCell;

use chrono::{DateTime, NaiveTime, Utc};
use nucleo_matcher::{Config, Matcher};
use web_time::{Duration, Instant};

use super::index::schedule::{DisplayedSchedule, source_query};
use super::{
    CategoryId, DateRange, DisplayTimeZone, Document, Event, EventId, EventSchedule, EventStatus,
    EventTimeZone, Year,
};
use crate::text::Language;
use text::{IndexedText, Rank};

#[cfg(test)]
mod tests;
mod text;

pub use text::{QueryError, SearchQuery};

const EVENTS_PER_STEP: usize = 512;
const CLOCK_CHECK_INTERVAL: usize = 16;
const STEP_BUDGET: Duration = Duration::from_millis(3);

#[derive(Clone, Copy)]
pub enum HighlightField {
    Title,
    Snippet,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum CalendarScope {
    #[default]
    All,
    Visible,
    Category(CategoryId),
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum StatusScope {
    #[default]
    Any,
    Value(Option<EventStatus>),
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct SearchFilters {
    pub dates: Option<DateRange>,
    pub calendars: CalendarScope,
    pub after: Option<NaiveTime>,
    pub status: StatusScope,
}

impl SearchFilters {
    fn includes(self, event: &Event, document: &Document) -> bool {
        let calendar = match self.calendars {
            CalendarScope::All => true,
            CalendarScope::Visible => document.visible_category(event.category).is_some(),
            CalendarScope::Category(id) => event.category == id,
        };
        calendar
            && match self.status {
                StatusScope::Any => true,
                StatusScope::Value(status) => event.status == status,
            }
    }
}

pub struct SearchHit {
    pub event: EventId,
    pub schedule: DisplayedSchedule,
    snippet_field: Option<usize>,
    position: usize,
    rank: Rank,
    label: OnceCell<String>,
}

impl SearchHit {
    pub fn date_label(&self) -> &str {
        self.label.get_or_init(|| self.schedule.to_string())
    }
    pub fn source<'a>(&self, document: &'a Document) -> &'a Event {
        &document.events[self.position]
    }
}

/// Буферы и UTF-32 представления переиспользуются; проход выполняется небольшими порциями на обеих платформах.
pub struct SearchIndex {
    records: Vec<IndexedText>,
    results: Vec<SearchHit>,
    matcher: Matcher,
    language: Option<Language>,
    next: usize,
    pending: bool,
}

impl Default for SearchIndex {
    fn default() -> Self {
        Self {
            records: Vec::new(),
            results: Vec::new(),
            matcher: Matcher::new(Config::DEFAULT),
            language: None,
            next: 0,
            pending: false,
        }
    }
}

impl SearchIndex {
    pub fn invalidate(&mut self) {
        self.records.clear();
        self.language = None;
        self.restart();
    }

    pub fn restart(&mut self) {
        self.results.clear();
        self.next = 0;
        self.pending = true;
    }

    pub fn results(&self) -> &[SearchHit] {
        &self.results
    }

    pub const fn is_pending(&self) -> bool {
        self.pending
    }

    pub fn highlight(
        &mut self,
        query: &SearchQuery,
        hit: usize,
        field: HighlightField,
        output: &mut Vec<u32>,
    ) {
        output.clear();
        let hit = &self.results[hit];
        let record = &self.records[hit.position];
        let text = match field {
            HighlightField::Title => &record.title,
            HighlightField::Snippet => {
                let Some(field) = hit.snippet_field else {
                    return;
                };
                &record.fields[field].text
            }
        };
        query.highlight(text, &mut self.matcher, output);
    }

    pub fn snippet<'a>(&self, hit: usize, document: &'a Document) -> Option<&'a str> {
        let hit = &self.results[hit];
        let field = hit.snippet_field?;
        Some(
            self.records[hit.position].fields[field]
                .source
                .get(hit.source(document), document),
        )
    }

    pub fn step(
        &mut self,
        document: &Document,
        query: &SearchQuery,
        filters: SearchFilters,
        now: DateTime<Utc>,
    ) {
        if self.language != Some(document.language) {
            self.records.clear();
            self.language = Some(document.language);
            self.restart();
        }
        if !self.pending {
            return;
        }
        let dates = match (filters.dates, query.date) {
            (Some(range), Some(date)) if !range.contains(date) => {
                self.pending = false;
                return;
            }
            (_, Some(date)) => Some(DateRange::between(date, date)),
            (range, None) => range,
        };
        let accept = |schedule: &DisplayedSchedule| {
            filters
                .after
                .is_none_or(|after| schedule.start_time().is_some_and(|time| time >= after))
                && query
                    .time
                    .is_none_or(|time| schedule.start_time() == Some(time))
        };
        let start = self.next;
        let end = (start + EVENTS_PER_STEP).min(document.events.len());
        let started = Instant::now();
        let zone = document.display_timezone;
        for (offset, event) in document.events[start..end].iter().enumerate() {
            if offset > 0
                && offset.is_multiple_of(CLOCK_CHECK_INTERVAL)
                && started.elapsed() >= STEP_BUDGET
            {
                break;
            }
            let position = start + offset;
            self.next = position + 1;
            if self.records.len() == position {
                self.records.push(IndexedText::new(event, document));
            }
            if !filters.includes(event, document) {
                continue;
            }
            let Some((rank, snippet_field)) =
                query.score(&self.records[position], &mut self.matcher)
            else {
                continue;
            };
            let mut push = |schedule| {
                self.results.push(SearchHit {
                    event: event.id,
                    schedule,
                    snippet_field,
                    position,
                    rank,
                    label: OnceCell::new(),
                });
            };
            if let Some(dates) = dates {
                for schedule in event
                    .occurrences(source_query(dates))
                    .filter_map(|schedule| DisplayedSchedule::project(schedule, zone))
                    .filter(|schedule| schedule.occupied_dates().overlaps(dates))
                    .filter(accept)
                {
                    push(schedule);
                }
            } else if let Some(schedule) = representative(event, zone, now, accept) {
                push(schedule);
            }
        }
        self.pending = self.next < document.events.len();
        // Сортируем только завершённую выдачу: порядок строк не прыгает между порциями.
        if !self.pending {
            self.results.sort_unstable_by(|left, right| {
                right
                    .rank
                    .cmp(&left.rank)
                    .then_with(|| left.schedule.start_date().cmp(&right.schedule.start_date()))
                    .then_with(|| left.event.0.cmp(&right.event.0))
            });
        }
    }
}

fn representative(
    event: &Event,
    zone: DisplayTimeZone,
    now: DateTime<Utc>,
    accept: impl Fn(&DisplayedSchedule) -> bool,
) -> Option<DisplayedSchedule> {
    // Сначала ищем предстоящее или продолжающееся вхождение, затем последнее завершённое.
    let supported = DateRange::between(
        Year::clamped(i32::MIN).range().start(),
        Year::clamped(i32::MAX).range().end(),
    );
    let today = zone
        .wall_time(now)
        .date()
        .clamp(supported.start(), supported.end());
    let future = DateRange::between(today, supported.end());
    event
        .occurrences(source_query(future))
        .filter_map(|source| {
            DisplayedSchedule::project(source, zone).map(|schedule| (source, schedule))
        })
        .filter(|(_, schedule)| schedule.occupied_dates().overlaps(future))
        .find(|(source, schedule)| {
            accept(schedule) && continues_after(*source, *schedule, zone, now)
        })
        .map(|(_, schedule)| schedule)
        .or_else(|| {
            event
                .occurrences(supported)
                .filter_map(|schedule| DisplayedSchedule::project(schedule, zone))
                .filter(accept)
                .last()
        })
}

/// Абсолютное окончание различает два одинаковых настенных часа при переходе на зимнее время.
fn continues_after(
    source: EventSchedule,
    displayed: DisplayedSchedule,
    zone: DisplayTimeZone,
    now: DateTime<Utc>,
) -> bool {
    if source.timezone() == EventTimeZone::Floating {
        displayed.continues_after(zone.wall_time(now))
    } else {
        source
            .times()
            .and_then(|times| {
                source
                    .timezone()
                    .to_utc(source.dates().end().and_time(times.end))
            })
            .is_some_and(|end| end > now)
    }
}
