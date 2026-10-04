//! Подготовка harness и воспроизводимых данных; ошибка подготовки немедленно прерывает тест.

#![expect(
    clippy::missing_panics_doc,
    reason = "Это вспомогательные функции тестов, ошибки фикстур должны прерывать сценарий."
)]

use chrono::NaiveDate;
use egui_kittest::Harness;
use planner::{
    Planner,
    testing::{
        self, AppState, Availability, CalendarViewMode, CategoryId, DateRange, DisplayTimeZone,
        Document, Event, EventDetails, EventId, EventIdentity, EventSchedule, EventUid, Frequency,
        Recurrence, Title, Year,
    },
};

pub const STEP_DT: f32 = 1.0 / 60.0;
pub type AppHarness = Harness<'static, Option<Planner>>;

#[must_use]
pub fn date(value: &str) -> NaiveDate {
    value.parse().unwrap()
}

#[must_use]
pub fn range(start: &str, end: &str) -> DateRange {
    DateRange::new(date(start), date(end)).unwrap()
}

/// Фиксированные год и зона исключают зависимость тестов от часов и настроек машины.
#[must_use]
pub fn document() -> Document {
    Document {
        year: Year::try_from(2026).unwrap(),
        display_timezone: DisplayTimeZone::Utc,
        ..Document::default()
    }
}

/// Событие с воспроизводимым UID, независимое от демонстрационных данных приложения.
#[must_use]
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

#[must_use]
pub fn profile_document(count: u32) -> Document {
    let mut document = Document {
        view_mode: CalendarViewMode::Continuous,
        ..document()
    };
    let start = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
    let recurrence = Recurrence::new(
        Frequency::Yearly,
        std::num::NonZeroU16::new(1).unwrap(),
        None,
    )
    .unwrap();
    document.events = (0..count)
        .map(|index| {
            let date = start + chrono::Days::new(u64::from(index % 366));
            Event {
                id: EventId(u64::from(index) + 1),
                identity: EventIdentity {
                    uid: EventUid::try_from(format!("scroll-{index}@planner")).unwrap(),
                    exclusions: Vec::new().into(),
                },
                title: Title::try_from(format!("Profile event {index}")).unwrap(),
                schedule: EventSchedule::all_day(DateRange::between(date, date))
                    .with_recurrence(Some(recurrence))
                    .unwrap(),
                category: CategoryId(1),
                notes: String::new(),
                location: String::new(),
                link: None,
                status: None,
                availability: Availability::default(),
                details: EventDetails::default(),
            }
        })
        .collect();
    document
}

#[must_use]
pub fn harness(document: Document, size: egui::Vec2) -> AppHarness {
    let mut document = Some(document);
    Harness::builder()
        .with_size(size)
        .with_step_dt(STEP_DT)
        .build_ui_state(
            move |ui, planner: &mut Option<Planner>| {
                let planner = planner.get_or_insert_with(|| {
                    testing::from_document(document.take().unwrap(), ui.ctx())
                });
                testing::render(planner, ui);
            },
            None,
        )
}

/// Заимствованный снимок состояния уже инициализированного приложения.
#[must_use]
pub fn state(harness: &AppHarness) -> AppState<'_> {
    harness
        .state()
        .as_ref()
        .expect("Harness did not initialize the application")
        .inspect()
}
