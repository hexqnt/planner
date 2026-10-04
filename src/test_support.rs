use chrono::NaiveDate;

use crate::model::{
    Availability, CategoryId, DateRange, DisplayTimeZone, Document, Event, EventDetails, EventId,
    EventIdentity, EventSchedule, EventUid, Title, Year,
};

pub fn date(value: &str) -> NaiveDate {
    value.parse().unwrap()
}

pub fn range(start: &str, end: &str) -> DateRange {
    DateRange::new(date(start), date(end)).unwrap()
}

/// Фиксированные год и зона исключают зависимость тестов от часов и настроек машины.
pub fn document() -> Document {
    Document {
        year: Year::try_from(2026).unwrap(),
        display_timezone: DisplayTimeZone::Utc,
        ..Document::default()
    }
}

/// Событие с воспроизводимым UID, независимое от демонстрационных данных приложения.
pub fn event(id: u64, category: CategoryId, schedule: EventSchedule) -> Event {
    Event {
        id: EventId(id),
        identity: EventIdentity {
            uid: EventUid::try_from(format!("event-{id}@tests")).unwrap(),
            exclusions: Vec::new().into(),
        },
        title: Title::try_from("Test event").unwrap(),
        schedule,
        category,
        notes: String::new(),
        location: String::new(),
        link: None,
        status: None,
        availability: Availability::Busy,
        details: EventDetails::default(),
    }
}

pub fn text_rect(output: &egui::FullOutput, label: &str) -> egui::Rect {
    output
        .shapes
        .iter()
        .rev()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.text() == label => {
                Some(egui::Rect::from_min_size(text.pos, text.galley.size()))
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("Missing label: {label}"))
}
