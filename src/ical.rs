//! Граница обмена iCalendar: разбор сразу создаёт собственные типы планера.

use icalendar::{
    Component as _, EventLike as _,
    parser::{Component, Property},
};
use rustc_hash::FxHashSet;

use crate::model::{
    Availability, CategoryId, Document, Event, EventId, EventIdentity, EventStatus, EventUid, Title,
};

mod dates;
mod recurrence;
mod syntax;
mod zones;

#[derive(Debug)]
pub struct Error(String);

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

impl From<&str> for Error {
    fn from(value: &str) -> Self {
        Self(value.into())
    }
}

impl From<crate::model::InputError> for Error {
    fn from(value: crate::model::InputError) -> Self {
        Self(value.message(crate::text::Language::English).into())
    }
}

type Result<T> = std::result::Result<T, Error>;

/// Весь файл преобразуется до изменения документа; неподдерживаемое расписание не теряется молча.
pub fn import(input: &str, category: CategoryId) -> Result<Vec<Event>> {
    let unfolded = syntax::unfold(input.trim_start_matches('\u{feff}'));
    // Парсер крейта потребляет весь ввод и проверяет парность BEGIN/END.
    let roots = icalendar::parser::read_components(&unfolded).map_err(Error)?;
    let [calendar] = roots.as_slice() else {
        return Err("Expected one VCALENDAR".into());
    };
    if calendar.name != "VCALENDAR" {
        return Err("Expected one VCALENDAR".into());
    }
    if required(calendar, "VERSION")?.val != "2.0" {
        return Err("Unsupported iCalendar version".into());
    }
    if property(calendar, "METHOD")?.is_some_and(|value| value.val.as_str() != "PUBLISH") {
        return Err("Scheduling METHOD is not supported".into());
    }
    let mut uids = FxHashSet::default();
    let mut events = Vec::new();
    for component in &calendar.components {
        if component.name != "VEVENT" {
            continue;
        }
        let event = (|| {
            let uid = required(component, "UID")?.val.as_str();
            if !uids.insert(uid) {
                return Err("Duplicate UID or recurrence overrides are not supported".into());
            }
            parse_event(component, category, uid)
        })()
        .map_err(|error| Error(format!("VEVENT {}: {error}", events.len() + 1)))?;
        events.push(event);
    }
    if events.is_empty() {
        return Err("No VEVENT events found".into());
    }
    Ok(events)
}

fn parse_event(component: &Component<'_>, category: CategoryId, uid: &str) -> Result<Event> {
    for unsupported in ["RECURRENCE-ID", "RDATE", "DURATION", "EXRULE"] {
        if property(component, unsupported)?.is_some() {
            return Err(Error(format!("{unsupported} is not supported")));
        }
    }
    let uid = EventUid::try_from(uid.to_owned())?;
    let start = dates::parse(required(component, "DTSTART")?)?;
    let end = property(component, "DTEND")?
        .map(dates::parse)
        .transpose()?;
    let schedule = dates::schedule(start, end)?;
    let rule = property(component, "RRULE")?
        .map(|property| recurrence::parse(property.val.as_str(), schedule))
        .transpose()?;
    let schedule = schedule.with_recurrence(rule)?;
    let mut exclusions = Vec::new();
    for property in component
        .properties
        .iter()
        .filter(|property| property.name.as_str().eq_ignore_ascii_case("EXDATE"))
    {
        for value in dates::exclusions(property, start)? {
            exclusions.push(value?);
        }
    }
    let title =
        property(component, "SUMMARY")?.map_or("Untitled event", |property| property.val.as_str());
    let title = Title::try_from(if title.trim().is_empty() {
        "Untitled event"
    } else {
        title
    })?;
    let status = property(component, "STATUS")?
        .map(|property| match property.val.as_str() {
            "TENTATIVE" => Ok(EventStatus::Tentative),
            "CONFIRMED" => Ok(EventStatus::Confirmed),
            "CANCELLED" => Ok(EventStatus::Cancelled),
            _ => Err(Error::from("Invalid STATUS")),
        })
        .transpose()?;
    let availability = match property(component, "TRANSP")?.map(|property| property.val.as_str()) {
        None | Some("OPAQUE") => Availability::Busy,
        Some("TRANSPARENT") => Availability::Free,
        Some(_) => return Err("Invalid TRANSP".into()),
    };
    Ok(Event {
        id: EventId(0),
        identity: EventIdentity {
            uid,
            exclusions: exclusions.into(),
        },
        title,
        schedule,
        category,
        notes: text(component, "DESCRIPTION")?,
        location: text(component, "LOCATION")?,
        link: property(component, "URL")?
            .map(|property| property.val.as_str().parse())
            .transpose()?,
        status,
        availability,
        details: crate::model::EventDetails::default(),
    })
}

fn property<'a, 'i>(component: &'a Component<'i>, name: &str) -> Result<Option<&'a Property<'i>>> {
    let mut matching = component
        .properties
        .iter()
        .filter(|property| property.name.as_str().eq_ignore_ascii_case(name));
    let result = matching.next();
    if matching.next().is_some() {
        return Err(Error(format!("Duplicate {name}")));
    }
    Ok(result)
}

fn required<'a, 'i>(component: &'a Component<'i>, name: &str) -> Result<&'a Property<'i>> {
    property(component, name)?.ok_or_else(|| Error(format!("Missing {name}")))
}

fn text(component: &Component<'_>, name: &str) -> Result<String> {
    Ok(property(component, name)?
        .map_or_else(String::new, |property| property.val.as_str().to_owned()))
}

/// Экспортируются исходные события всех календарей, включая скрытые; повторы остаются правилами.
pub fn export(document: &Document) -> Result<String> {
    const CALENDAR_END: &str = "END:VCALENDAR\r\n";
    let mut calendar = icalendar::Calendar {
        properties: icalendar::Property::from_array([
            ("VERSION", "2.0"),
            ("PRODID", "-//planner//Year planner//EN"),
            ("CALSCALE", "GREGORIAN"),
        ]),
        components: Vec::with_capacity(document.events.len()),
    };
    let mut timezones = FxHashSet::default();
    for event in &document.events {
        let mut output = icalendar::Event::new();
        output
            .uid(event.identity.uid.as_str())
            .summary(event.title.get())
            .description(&event.notes)
            .location(&event.location);
        for value in dates::properties(event.schedule)? {
            output.append_property(value);
        }
        if let Some(rule) = event.schedule.recurrence() {
            output.add_property("RRULE", recurrence::format(rule, event.schedule)?);
        }
        for start in event.identity.exclusions.iter() {
            output.append_multi_property(dates::exclusion_property(start));
        }
        if let Some(link) = &event.link {
            output.url(link.as_str());
        }
        if let Some(status) = event.status {
            output.add_property(
                "STATUS",
                match status {
                    EventStatus::Tentative => "TENTATIVE",
                    EventStatus::Confirmed => "CONFIRMED",
                    EventStatus::Cancelled => "CANCELLED",
                },
            );
        }
        output.add_property(
            "TRANSP",
            match event.availability {
                Availability::Busy => "OPAQUE",
                Availability::Free => "TRANSPARENT",
            },
        );
        if let crate::model::EventTimeZone::Named(zone) = event.schedule.timezone() {
            timezones.insert(zone);
        }
        calendar.push(output);
    }
    // Other в icalendar добавляет UID/DTSTAMP и к VTIMEZONE, поэтому определения зон записываются отдельно.
    let mut output = calendar.to_string();
    let end = output
        .strip_suffix(CALENDAR_END)
        .ok_or("Invalid calendar serialization")?
        .len();
    output.truncate(end);
    let mut timezones: Vec<_> = timezones.into_iter().collect();
    timezones.sort_unstable_by_key(|zone| zone.name());
    for zone in timezones {
        zones::append(&mut output, zone);
    }
    output.push_str(CALENDAR_END);
    Ok(output)
}

#[cfg(test)]
mod tests;
