use chrono::{Datelike as _, Local, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::text::{Language, Name};

pub use display_timezone::{DisplayTimeZone, RecentTimeZones};
pub use document::{CalendarViewMode, Document, EventListPosition};
pub use event::{
    Availability, Email, Event, EventDetails, EventIdentity, EventLink, EventSchedule, EventStatus,
    EventTimeZone, EventTimes, EventUid, Frequency, OccurrenceStart, Recurrence, Reminder,
    Weekdays,
};
pub use index::EventIndex;
pub use search::{
    CalendarScope, HighlightField, QueryError, SearchFilters, SearchIndex, SearchQuery, StatusScope,
};

mod display_timezone;
mod document;
mod event;
mod index;
mod search;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputError {
    Year,
    ReversedRange,
    EmptyTitle,
    LongTitle,
    StartDate,
    EndDate,
    UnknownCategory,
    StartTime,
    EndTime,
    ReversedTime,
    EventLink,
    Participant,
    Attachment,
    Reminder,
    RecurrenceUntil,
    RecurrencePattern,
}

impl InputError {
    #[must_use]
    pub const fn message(self, language: Language) -> &'static str {
        let (ru, en) = match self {
            Self::Participant => (
                "Некорректный email участника. Например: name@example.com",
                "Invalid participant email. Example: name@example.com",
            ),
            Self::Attachment => (
                "Вложения должны быть полными HTTP(S)-ссылками",
                "Attachments must be complete HTTP(S) links",
            ),
            Self::Reminder => (
                "Напоминание: от 0 до 40320 минут до начала",
                "Reminder must be 0 to 40320 minutes before start",
            ),
            Self::RecurrencePattern => (
                "Правило повторов некорректно: количество и конечная дата несовместимы; дни недели доступны только для недельных повторов и должны включать начальный день",
                "Invalid recurrence: count and end date are mutually exclusive; weekdays require weekly recurrence and must include the start day",
            ),
            Self::RecurrenceUntil => (
                "Конец повторов должен быть не раньше начала события; формат ГГГГ-ММ-ДД",
                "Repeat end must not precede event start; use YYYY-MM-DD",
            ),

            Self::StartTime => (
                "Некорректное время начала. Формат: ЧЧ:ММ",
                "Invalid start time. Use HH:MM",
            ),
            Self::EndTime => (
                "Некорректное время окончания. Формат: ЧЧ:ММ",
                "Invalid end time. Use HH:MM",
            ),
            Self::ReversedTime => (
                "Окончание не может быть раньше начала",
                "End must not precede start",
            ),
            Self::EventLink => (
                "Введите полную ссылку с http:// или https://",
                "Enter a complete link with http:// or https://",
            ),
            Self::Year => (
                "Год должен быть от 1900 до 2100",
                "Year must be between 1900 and 2100",
            ),
            Self::ReversedRange => (
                "Конечная дата не может быть раньше начальной",
                "End date must not precede start date",
            ),
            Self::EmptyTitle => ("Введите название события", "Event title is required"),
            Self::LongTitle => (
                "Название события слишком длинное",
                "Event title is too long",
            ),
            Self::StartDate => (
                "Некорректная начальная дата. Формат: ГГГГ-ММ-ДД",
                "Invalid start date. Use YYYY-MM-DD",
            ),
            Self::UnknownCategory => (
                "Календарь больше не существует. Выберите другой.",
                "The calendar no longer exists. Choose another one.",
            ),
            Self::EndDate => (
                "Некорректная конечная дата. Формат: ГГГГ-ММ-ДД",
                "Invalid end date. Use YYYY-MM-DD",
            ),
        };
        language.text(ru, en)
    }
}

impl std::error::Error for InputError {}

impl std::fmt::Display for InputError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.message(Language::English))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "i32", into = "i32")]
pub struct Year(i32);

impl Year {
    #[must_use]
    pub const fn get(self) -> i32 {
        self.0
    }

    #[must_use]
    pub fn current() -> Self {
        Self::clamped(Local::now().year())
    }

    #[must_use]
    pub fn clamped(value: i32) -> Self {
        Self(value.clamp(holidays_ru::MIN_YEAR, holidays_ru::MAX_YEAR))
    }

    /// # Panics
    /// Паника означает нарушение инварианта поддерживаемого года.
    #[must_use]
    pub const fn range(self) -> DateRange {
        DateRange {
            start: NaiveDate::from_ymd_opt(self.0, 1, 1).expect("Valid year"),
            end: NaiveDate::from_ymd_opt(self.0, 12, 31).expect("Valid year"),
        }
    }

    #[must_use]
    pub fn step(self, delta: i32) -> Option<Self> {
        self.0
            .checked_add(delta)
            .and_then(|year| Self::try_from(year).ok())
    }
}

impl TryFrom<i32> for Year {
    type Error = InputError;
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        if (holidays_ru::MIN_YEAR..=holidays_ru::MAX_YEAR).contains(&value) {
            Ok(Self(value))
        } else {
            Err(InputError::Year)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "RangeInput", into = "RangeInput")]
pub struct DateRange {
    start: NaiveDate,
    end: NaiveDate,
}

impl DateRange {
    /// # Errors
    /// Возвращает ошибку для неподдерживаемого года или обратного диапазона.
    pub fn new(start: NaiveDate, end: NaiveDate) -> Result<Self, InputError> {
        Year::try_from(start.year())?;
        Year::try_from(end.year())?;
        if start > end {
            return Err(InputError::ReversedRange);
        }
        Ok(Self { start, end })
    }

    /// # Panics
    /// Паникует, если год одной из дат вне поддерживаемого диапазона.
    #[must_use]
    pub fn between(a: NaiveDate, b: NaiveDate) -> Self {
        Self::new(a.min(b), a.max(b)).expect("Selection dates are within the supported range")
    }

    #[must_use]
    pub const fn start(self) -> NaiveDate {
        self.start
    }
    #[must_use]
    pub const fn end(self) -> NaiveDate {
        self.end
    }
    #[must_use]
    pub fn contains(self, date: NaiveDate) -> bool {
        (self.start..=self.end).contains(&date)
    }
    #[must_use]
    pub fn overlaps(self, other: Self) -> bool {
        self.start <= other.end && other.start <= self.end
    }
    #[must_use]
    pub const fn days(self) -> i64 {
        self.end.signed_duration_since(self.start).num_days() + 1
    }
}

impl std::fmt::Display for DateRange {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.start.format("%d.%m.%Y"))?;
        if self.end != self.start {
            write!(formatter, " — {}", self.end.format("%d.%m.%Y"))?;
        }
        Ok(())
    }
}

impl TryFrom<RangeInput> for DateRange {
    type Error = InputError;
    fn try_from(value: RangeInput) -> Result<Self, Self::Error> {
        Self::new(value.start, value.end)
    }
}

#[derive(Serialize, Deserialize)]
struct RangeInput {
    start: NaiveDate,
    end: NaiveDate,
}

impl From<DateRange> for RangeInput {
    fn from(value: DateRange) -> Self {
        Self {
            start: value.start,
            end: value.end,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct Title(String);

impl Title {
    #[must_use]
    pub fn get(&self) -> &str {
        &self.0
    }

    fn parse(value: &str) -> Result<&str, InputError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(InputError::EmptyTitle);
        }
        if trimmed.len() > 500 {
            return Err(InputError::LongTitle);
        }
        Ok(trimmed)
    }
}

impl TryFrom<&str> for Title {
    type Error = InputError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl TryFrom<String> for Title {
    type Error = InputError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let trimmed = Self::parse(&value)?;
        if trimmed.len() == value.len() {
            Ok(Self(value))
        } else {
            Ok(Self(trimmed.into()))
        }
    }
}

impl std::str::FromStr for Title {
    type Err = InputError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value).map(|trimmed| Self(trimmed.into()))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CategoryId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventId(pub u64);

#[derive(Serialize, Deserialize)]
pub struct Category {
    pub id: CategoryId,
    pub name: Name,
    pub enabled: bool,
    pub color: [u8; 3],
}

#[derive(Serialize, Deserialize)]
pub struct Group {
    pub name: Name,
    pub enabled: bool,
    pub expanded: bool,
    pub categories: Vec<Category>,
}

impl From<Year> for i32 {
    fn from(value: Year) -> Self {
        value.0
    }
}

#[cfg(test)]
mod tests;
