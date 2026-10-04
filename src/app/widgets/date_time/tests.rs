use super::*;

#[test]
fn dates_accept_both_formats_and_reject_impossible_days() {
    assert_eq!(parse_date(" 29.02.2024 "), parse_date("2024-02-29"));
    for text in ["29.02.2025", "2025-04-31", "", "invalid", "2025-13-01"] {
        assert!(parse_date(text).is_err(), "{text}");
    }
}

#[test]
fn time_accepts_short_input_and_preserves_precision() {
    assert_eq!(parse_time("930"), parse_time("09:30"));
    assert_eq!(parse_time("0930"), parse_time("09:30"));
    assert_eq!(parse_time(" 9:30 "), parse_time("09:30"));
    let time = parse_time("13:00:01.125").unwrap();
    assert_eq!(format_time(time), "13:00:01.125");
    for text in ["2400", "960", "25:00", "13:60", "", "bad"] {
        assert!(parse_time(text).is_err(), "{text}");
    }
}

#[test]
fn stepping_dates_clamps_months_and_handles_leap_years() {
    let value = Value::Date(parse_date("2024-01-31").unwrap());
    assert_eq!(
        value
            .step("31.01.2024", 3, true)
            .unwrap()
            .format(Language::Russian),
        "29.02.2024"
    );
    assert_eq!(
        value
            .step("2024-01-31", 5, true)
            .unwrap()
            .format(Language::English),
        "2024-02-29"
    );
    let leap = Value::Date(parse_date("2024-02-29").unwrap());
    assert_eq!(
        leap.step("29.02.2024", 6, true)
            .unwrap()
            .format(Language::Russian),
        "28.02.2025"
    );
    assert_eq!(
        leap.step("29.02.2024", 0, true)
            .unwrap()
            .format(Language::Russian),
        "01.03.2024"
    );
}

#[test]
fn stepping_time_wraps_and_preserves_fractional_seconds() {
    let value = Value::Time(parse_time("23:59:01.125").unwrap());
    assert_eq!(
        value
            .step("23:59:01.125", 3, true)
            .unwrap()
            .format(Language::English),
        "00:00:01.125"
    );
    assert_eq!(
        value
            .step("23:59:01.125", 6, false)
            .unwrap()
            .format(Language::English),
        "23:59:00.125"
    );
}
