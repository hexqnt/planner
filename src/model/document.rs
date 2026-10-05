use chrono::NaiveDate;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};

use crate::{
    calendar::Region,
    text::{Language, LanguageMode, Name},
};

use super::{
    Category, CategoryId, DateRange, DisplayTimeZone, Event, EventId, Group, RecentTimeZones,
    Title, Year,
};

#[derive(Debug)]
enum DocumentError {
    Version,
    DuplicateCategory,
    DuplicateEvent,
    DuplicateUid,
    InvalidExclusion,
    UnknownCategory,
}

impl std::fmt::Display for DocumentError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Version => "Unsupported document version",
            Self::DuplicateCategory => "Duplicate category ID",
            Self::DuplicateEvent => "Duplicate event ID",
            Self::DuplicateUid => "Duplicate event UID",
            Self::InvalidExclusion => "Exclusions must match the event schedule type",
            Self::UnknownCategory => "Event references an unknown category",
        })
    }
}

impl std::error::Error for DocumentError {}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CalendarViewMode {
    #[default]
    SingleYear,
    Continuous,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventListPosition {
    #[default]
    Left,
    Right,
}

#[derive(Serialize, Deserialize)]
#[serde(try_from = "DocumentInput")]
pub struct Document {
    pub version: u8,
    pub year: Year,
    pub view_mode: CalendarViewMode,
    pub event_list_position: EventListPosition,
    pub display_timezone: DisplayTimeZone,
    pub recent_timezones: RecentTimeZones,
    pub last_event_category: Option<CategoryId>,
    pub region: Region,
    pub language: Language,
    pub language_mode: LanguageMode,
    pub dark: bool,
    pub vacation: crate::vacation::Settings,
    pub groups: Vec<Group>,
    pub events: Vec<Event>,
}

impl Document {
    pub fn resolve_language(&mut self) {
        if self.language_mode == LanguageMode::Auto {
            self.language = Language::system();
        }
    }

    pub fn remove_group(&mut self, index: usize) {
        if index >= self.groups.len() {
            return;
        }
        let group = self.groups.remove(index);
        let removed: FxHashSet<_> = group
            .categories
            .iter()
            .map(|category| category.id)
            .collect();
        self.events
            .retain(|event| !removed.contains(&event.category));
        if self
            .last_event_category
            .is_some_and(|id| removed.contains(&id))
        {
            self.last_event_category = None;
        }
    }

    pub fn remove_category(&mut self, id: CategoryId) {
        for group in &mut self.groups {
            group.categories.retain(|category| category.id != id);
        }
        self.events.retain(|event| event.category != id);
        if self.last_event_category == Some(id) {
            self.last_event_category = None;
        }
    }

    pub fn categories(&self) -> impl Iterator<Item = &Category> {
        self.groups.iter().flat_map(|group| &group.categories)
    }

    #[must_use]
    pub fn category(&self, id: CategoryId) -> Option<&Category> {
        self.categories().find(|category| category.id == id)
    }

    #[must_use]
    pub fn visible_category(&self, id: CategoryId) -> Option<&Category> {
        self.visible_categories().find(|category| category.id == id)
    }

    pub fn visible_categories(&self) -> impl Iterator<Item = &Category> {
        self.groups
            .iter()
            .filter(|group| group.enabled)
            .flat_map(|group| &group.categories)
            .filter(|category| category.enabled)
    }

    #[must_use]
    pub fn next_category_id(&self) -> CategoryId {
        CategoryId(first_unused_id(
            self.categories().map(|category| category.id.0),
        ))
    }

    #[must_use]
    pub fn next_event_id(&self) -> EventId {
        EventId(first_unused_id(self.events.iter().map(|event| event.id.0)))
    }

    /// Десериализация проверяет версию и все ссылки независимо от способа загрузки.
    ///
    /// # Errors
    /// Возвращает ошибку для некорректного JSON, версии документа или ссылок на календари.
    pub fn parse(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Совпадающий UID обновляет событие, сохраняя локальные ID и принадлежность календарю.
    ///
    /// # Panics
    /// Паника возможна при исчерпании пространства идентификаторов событий.
    pub fn merge_events(&mut self, events: Vec<Event>) {
        if events.is_empty() {
            return;
        }
        // Индекс владеет UID: замена событий и рост вектора не должны инвалидировать ключи.
        let mut lookup: FxHashMap<_, _> = self
            .events
            .iter()
            .enumerate()
            .map(|(position, event)| (event.identity.uid.clone(), position))
            .collect();
        let occupied: FxHashSet<_> = self.events.iter().map(|event| event.id.0).collect();
        let limit = u64::try_from(occupied.len() + events.len()).expect("Event count fits u64") + 1;
        let mut available = (1..=limit).filter(|id| !occupied.contains(id));
        for mut event in events {
            if let Some(&position) = lookup.get(&event.identity.uid) {
                let existing = &mut self.events[position];
                event.id = existing.id;
                event.category = existing.category;
                *existing = event;
            } else {
                event.id = EventId(available.next().expect("Available event ID"));
                lookup.insert(event.identity.uid.clone(), self.events.len());
                self.events.push(event);
            }
        }
    }

    ///
    /// # Panics
    /// Паника означает нарушение инварианта поддерживаемого года или демонстрационных дат.
    pub fn add_examples(&mut self) {
        let examples = [
            (1, 23, 23, 2, "Встреча", "Meeting"),
            (2, 24, 28, 7, "Зимний отпуск", "Winter vacation"),
            (4, 6, 10, 1, "Запуск проекта", "Project launch"),
            (6, 1, 5, 7, "Отпуск", "Vacation"),
            (7, 20, 24, 3, "Дежурство", "On call"),
            (8, 10, 14, 11, "Курс", "Course"),
            (9, 21, 25, 9, "Путешествие", "Travel"),
            (10, 15, 15, 6, "День рождения", "Birthday"),
            (11, 23, 27, 7, "Осенний отпуск", "Autumn vacation"),
            (12, 28, 31, 1, "Завершение проекта", "Project wrap-up"),
        ];
        for (month, start, end, category, ru, en) in examples {
            let category = CategoryId(category);
            if self.category(category).is_none() {
                continue;
            }
            let date = |day| {
                NaiveDate::from_ymd_opt(self.year.get(), month, day).expect("Valid example date")
            };
            self.events.push(Event {
                id: self.next_event_id(),
                identity: super::EventIdentity::default(),
                title: Title::try_from(self.language.text(ru, en)).expect("Nonempty title"),
                schedule: super::EventSchedule::all_day(DateRange::between(date(start), date(end))),
                category,
                notes: String::new(),
                location: String::new(),
                link: None,
                status: None,
                availability: super::Availability::default(),
                details: super::EventDetails::default(),
            });
        }
    }
}

impl Default for Document {
    fn default() -> Self {
        let mut id = 0;
        let mut group = |ru, en, items: &[(&str, &str, [u8; 3])]| Group {
            name: Name::new(ru, en).expect("Nonempty built-in names"),
            enabled: true,
            expanded: true,
            categories: items
                .iter()
                .map(|&(ru, en, color)| {
                    id += 1;
                    Category {
                        id: CategoryId(id),
                        name: Name::new(ru, en).expect("Nonempty built-in names"),
                        enabled: true,
                        color,
                    }
                })
                .collect(),
        };
        Self {
            version: 1,
            year: Year::current(),
            view_mode: CalendarViewMode::default(),
            event_list_position: EventListPosition::default(),
            display_timezone: DisplayTimeZone::default(),
            recent_timezones: RecentTimeZones::default(),
            last_event_category: None,
            region: Region::Federal,
            language: Language::Russian,
            language_mode: LanguageMode::Manual,
            dark: false,
            vacation: crate::vacation::Settings::default(),
            groups: vec![
                group(
                    "Работа",
                    "Work",
                    &[
                        ("Проекты", "Projects", [39, 133, 245]),
                        ("Встречи", "Meetings", [163, 105, 239]),
                        ("Дежурства", "On call", [244, 163, 66]),
                        ("Командировки", "Business trips", [36, 181, 192]),
                    ],
                ),
                group(
                    "Семья",
                    "Family",
                    &[
                        ("Праздники", "Celebrations", [245, 83, 89]),
                        ("Дни рождения", "Birthdays", [232, 122, 179]),
                        ("Отпуск", "Vacation", [76, 189, 103]),
                        ("Здоровье", "Health", [236, 182, 68]),
                    ],
                ),
                group(
                    "Поездки",
                    "Trips",
                    &[
                        ("Путешествия", "Travel", [109, 172, 239]),
                        ("Выходные", "Getaways", [157, 111, 221]),
                    ],
                ),
                group(
                    "Учёба",
                    "Learning",
                    &[
                        ("Курсы", "Courses", [155, 101, 235]),
                        ("Экзамены", "Exams", [244, 87, 87]),
                        ("Саморазвитие", "Self development", [85, 192, 119]),
                    ],
                ),
            ],
            events: Vec::new(),
        }
    }
}

impl TryFrom<DocumentInput> for Document {
    type Error = DocumentError;

    fn try_from(document: DocumentInput) -> Result<Self, Self::Error> {
        if document.version != 1 {
            return Err(DocumentError::Version);
        }
        let mut category_ids = FxHashSet::default();
        for group in &document.groups {
            for category in &group.categories {
                if !category_ids.insert(category.id) {
                    return Err(DocumentError::DuplicateCategory);
                }
            }
        }
        let mut event_ids = FxHashSet::default();
        let mut identities = FxHashSet::default();
        for event in &document.events {
            if !category_ids.contains(&event.category) {
                return Err(DocumentError::UnknownCategory);
            }
            if !identities.insert(event.identity.uid.as_str()) {
                return Err(DocumentError::DuplicateUid);
            }
            if event
                .identity
                .exclusions
                .iter()
                .any(|start| !start.matches_schedule_type(event.schedule))
            {
                return Err(DocumentError::InvalidExclusion);
            }
            if !event_ids.insert(event.id) {
                return Err(DocumentError::DuplicateEvent);
            }
        }
        Ok(Self {
            version: document.version,
            year: document.year,
            view_mode: document.view_mode,
            event_list_position: document.event_list_position,
            display_timezone: document.display_timezone,
            recent_timezones: document.recent_timezones,
            last_event_category: document
                .last_event_category
                .filter(|id| category_ids.contains(id)),
            region: document.region,
            language: document.language,
            language_mode: document.language_mode,
            dark: document.dark,
            vacation: document.vacation,
            groups: document.groups,
            events: document.events,
        })
    }
}

// Входная форма не выходит за пределы модуля: Document создаётся только после проверки ссылок.
#[derive(Deserialize)]
struct DocumentInput {
    version: u8,
    year: Year,
    #[serde(default)]
    view_mode: CalendarViewMode,
    #[serde(default)]
    event_list_position: EventListPosition,
    #[serde(default)]
    display_timezone: DisplayTimeZone,
    #[serde(default)]
    recent_timezones: RecentTimeZones,
    #[serde(default)]
    last_event_category: Option<CategoryId>,
    region: Region,
    language: Language,
    #[serde(default)]
    language_mode: LanguageMode,
    dark: bool,
    #[serde(default)]
    vacation: crate::vacation::Settings,
    groups: Vec<Group>,
    events: Vec<Event>,
}

fn first_unused_id(ids: impl Iterator<Item = u64>) -> u64 {
    let used: FxHashSet<_> = ids.collect();
    (1..=u64::try_from(used.len()).expect("ID count fits u64") + 1)
        .find(|id| !used.contains(id))
        .expect("Available ID")
}

#[cfg(test)]
mod tests;
