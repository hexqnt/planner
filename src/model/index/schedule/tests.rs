use super::*;
use crate::test_support::range;

#[test]
fn all_day_labels_omit_duplicate_dates_and_preserve_distinct_endpoints() {
    for (start, end, expected) in [
        ("2026-10-04", "2026-10-04", "04.10.2026"),
        ("2026-10-04", "2026-10-05", "04.10.2026 — 05.10.2026"),
        ("2026-12-31", "2027-01-01", "31.12.2026 — 01.01.2027"),
    ] {
        let dates = range(start, end);
        assert_eq!(dates.to_string(), expected);
        assert_eq!(DisplayedSchedule::AllDay(dates).to_string(), expected);
    }
}

#[test]
fn timed_labels_only_repeat_dates_when_the_end_is_on_another_day() {
    for (start, end, expected) in [
        (
            "2026-10-04 09:00:00",
            "2026-10-04 10:30:00",
            "04.10.2026 09:00 — 10:30",
        ),
        (
            "2026-10-04 09:00:00",
            "2026-10-04 09:00:00",
            "04.10.2026 09:00",
        ),
        (
            "2026-10-04 23:00:00",
            "2026-10-05 00:00:00",
            "04.10.2026 23:00 — 05.10.2026 00:00",
        ),
        (
            "2026-10-04 09:00:00",
            "2026-10-05 09:00:00",
            "04.10.2026 09:00 — 05.10.2026 09:00",
        ),
        (
            "2026-12-31 23:30:00",
            "2027-01-01 00:30:00",
            "31.12.2026 23:30 — 01.01.2027 00:30",
        ),
        (
            "2026-10-25 02:45:00",
            "2026-10-25 02:15:00",
            "25.10.2026 02:45 — 02:15",
        ),
    ] {
        let start = NaiveDateTime::parse_from_str(start, "%Y-%m-%d %H:%M:%S").unwrap();
        let end = NaiveDateTime::parse_from_str(end, "%Y-%m-%d %H:%M:%S").unwrap();
        let schedule = DisplayedSchedule::Timed {
            start,
            end,
            occupied: DateRange::between(start.date(), end.date()),
        };
        assert_eq!(schedule.to_string(), expected);
    }
}
