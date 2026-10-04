use super::*;
use crate::test_support::{date, range};

#[test]
fn ranges_are_inclusive_and_overlap_symmetrically() {
    let span = range("2026-12-28", "2027-01-08");
    assert_eq!(span.days(), 12);
    for (day, contains) in [
        ("2026-12-27", false),
        ("2026-12-28", true),
        ("2027-01-08", true),
        ("2027-01-09", false),
    ] {
        assert_eq!(span.contains(date(day)), contains, "{day}");
    }
    for (other, overlaps) in [
        (range("2026-12-26", "2026-12-27"), false),
        (range("2026-12-26", "2026-12-28"), true),
        (range("2026-12-29", "2027-01-01"), true),
        (range("2026-12-01", "2027-01-31"), true),
        (range("2027-01-08", "2027-01-10"), true),
        (range("2027-01-09", "2027-01-10"), false),
    ] {
        assert_eq!(span.overlaps(other), overlaps, "{other:?}");
        assert_eq!(other.overlaps(span), overlaps, "{other:?}");
    }
    assert_eq!(range("2026-10-03", "2026-10-03").days(), 1);
    assert_eq!(DateRange::between(span.end(), span.start()), span);
    assert_eq!(
        serde_json::from_value::<DateRange>(serde_json::to_value(span).unwrap()).unwrap(),
        span
    );
}

#[test]
fn ranges_reject_reversed_or_unsupported_dates_on_both_input_paths() {
    for (start, end, error) in [
        ("2027-01-08", "2026-12-28", InputError::ReversedRange),
        ("1899-12-31", "1900-01-01", InputError::Year),
        ("2100-12-31", "2101-01-01", InputError::Year),
    ] {
        assert_eq!(DateRange::new(date(start), date(end)), Err(error));
        let error = error.to_string();
        let restored =
            serde_json::from_value::<DateRange>(serde_json::json!({"start": start, "end": end}))
                .unwrap_err();
        assert!(
            restored.to_string().contains(&error),
            "{start}..{end}: {restored}"
        );
    }
    assert!(DateRange::new(date("1900-01-01"), date("2100-12-31")).is_ok());
}

#[test]
fn years_parse_and_clamp_at_supported_boundaries() {
    for (value, expected) in [
        (i32::MIN, 1900),
        (1899, 1900),
        (1900, 1900),
        (2026, 2026),
        (2100, 2100),
        (2101, 2100),
        (i32::MAX, 2100),
    ] {
        assert_eq!(Year::clamped(value).get(), expected, "{value}");
        let parsed = Year::try_from(value);
        let restored = serde_json::from_value::<Year>(serde_json::json!(value));
        if value == expected {
            let year = parsed.unwrap();
            assert_eq!(year.get(), expected);
            assert_eq!(restored.unwrap(), year);
            assert_eq!(serde_json::to_value(year).unwrap(), value);
        } else {
            assert_eq!(parsed, Err(InputError::Year));
            assert!(restored.is_err(), "{value}");
        }
    }
}

#[test]
fn year_steps_handle_boundaries_and_integer_overflow() {
    for (year, delta, expected) in [
        (1900, -1, None),
        (2100, 1, None),
        (2026, i32::MAX, None),
        (2026, i32::MIN, None),
        (1900, 0, Some(1900)),
        (2100, -200, Some(1900)),
        (1900, 200, Some(2100)),
        (2026, 1, Some(2027)),
    ] {
        assert_eq!(
            Year::try_from(year).unwrap().step(delta).map(Year::get),
            expected
        );
    }
}

#[test]
fn year_ranges_follow_gregorian_leap_rules() {
    for (year, days) in [
        (1900, 365),
        (2000, 366),
        (2024, 366),
        (2026, 365),
        (2100, 365),
    ] {
        let span = Year::try_from(year).unwrap().range();
        assert_eq!(span.start(), date(&format!("{year}-01-01")));
        assert_eq!(span.end(), date(&format!("{year}-12-31")));
        assert_eq!(span.days(), days, "{year}");
    }
}

#[test]
fn titles_normalize_whitespace_on_every_input_path() {
    for (value, expected) in [("  Отпуск  ", "Отпуск"), ("Vacation", "Vacation")] {
        for title in [
            Title::try_from(value).unwrap(),
            Title::try_from(value.to_owned()).unwrap(),
            value.parse::<Title>().unwrap(),
            serde_json::from_value::<Title>(serde_json::json!(value)).unwrap(),
        ] {
            assert_eq!(title.get(), expected);
            assert_eq!(serde_json::to_value(title).unwrap(), expected);
        }
    }
}

#[test]
fn titles_check_empty_input_and_utf8_byte_length_after_trimming() {
    for (value, expected) in [
        (String::new(), Err(InputError::EmptyTitle)),
        (" \n\t ".into(), Err(InputError::EmptyTitle)),
        ("x".repeat(500), Ok(())),
        ("x".repeat(501), Err(InputError::LongTitle)),
        (format!(" {} ", "я".repeat(250)), Ok(())),
        (format!("{}x", "я".repeat(250)), Err(InputError::LongTitle)),
    ] {
        for result in [
            Title::try_from(value.as_str()),
            Title::try_from(value.clone()),
            value.parse::<Title>(),
        ] {
            assert_eq!(result.map(|_| ()), expected, "{value:?}");
        }
        let restored = serde_json::from_value::<Title>(serde_json::json!(value));
        assert_eq!(restored.is_ok(), expected.is_ok(), "{value:?}");
    }
}
