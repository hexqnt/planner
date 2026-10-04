use super::*;
use crate::test_support::{date, range};

fn pay() -> Pay {
    Pay {
        salary: "40000".parse().unwrap(),
        average: None,
    }
}

#[test]
fn amount_parses_decimal_money_without_float_rounding() {
    for (input, expected) in [
        ("40 000", 4_000_000),
        ("1,25", 125),
        ("0.01", 1),
        (" 1\u{202f}234.5 ", 123_450),
        ("42949672.95", i64::from(u32::MAX)),
    ] {
        assert_eq!(input.parse::<Amount>().unwrap().cents(), expected);
    }
    for input in [
        "",
        "0",
        "-1",
        "NaN",
        "1e3",
        "1.001",
        "1.",
        ".1",
        "1,2.3",
        "1.2 3",
        "42949672.96",
        "999999999999999",
    ] {
        assert!(input.parse::<Amount>().is_err(), "{input}");
    }
}

#[test]
fn october_example_preserves_precision_and_extends_rest() {
    let estimate = Estimate::new(range("2026-10-03", "2026-10-09"), Region::Federal, pay());
    assert_eq!(estimate.paid_days, 7);
    assert_eq!(estimate.missed_workdays, 5);
    assert_eq!(estimate.vacation_pay, 955_631);
    assert_eq!(estimate.remaining_salary, 3_090_909);
    assert_eq!(estimate.income_change(), 46_540);
    assert_eq!(estimate.rest, range("2026-10-03", "2026-10-11"));
    assert!(!estimate.predicted);
    let weekdays = Estimate::new(range("2026-10-05", "2026-10-09"), Region::Federal, pay());
    assert_eq!(weekdays.paid_days, 5);
    assert_eq!(weekdays.rest, estimate.rest);
    assert_eq!(weekdays.income_change(), -226_497);
}

#[test]
fn holidays_are_unpaid_but_transferred_days_off_are_paid() {
    for (input, paid) in [("2026-01-01", 0), ("2026-01-09", 1), ("2026-12-31", 1)] {
        let day = date(input);
        let estimate = Estimate::new(DateRange::between(day, day), Region::Federal, pay());
        assert_eq!(estimate.paid_days, paid, "{input}");
        assert_eq!(estimate.missed_workdays, 0);
        assert_eq!(estimate.holidays, 1 - paid);
        assert_eq!(estimate.vacation_pay, pay().daily().payment(paid));
    }
    let federal = Estimate::new(range("2026-11-06", "2026-11-06"), Region::Federal, pay());
    let regional = Estimate::new(range("2026-11-06", "2026-11-06"), Region::Tatarstan, pay());
    assert_eq!(federal.paid_days, 1);
    assert_eq!(regional.paid_days, 0);
    assert_eq!(federal.missed_workdays, 1);
    assert_eq!(regional.missed_workdays, 0);
}

#[test]
fn manual_average_cross_month_and_year_ranges_use_each_months_norm() {
    let pay = Pay {
        average: Some("2000".parse().unwrap()),
        ..pay()
    };
    let estimate = Estimate::new(range("2026-07-31", "2026-08-03"), Region::Federal, pay);
    assert_eq!(estimate.paid_days, 4);
    assert_eq!(estimate.missed_workdays, 2);
    assert_eq!(estimate.vacation_pay, 800_000);
    assert_eq!(estimate.months.len(), 2);
    assert_eq!(estimate.months[0].loss, 173_913);
    assert_eq!(estimate.months[1].loss, 190_476);
    assert_eq!(estimate.income_change(), 435_611);
    let cross_year = Estimate::new(range("2025-12-30", "2026-01-10"), Region::Federal, pay);
    assert_eq!(cross_year.months.len(), 2);
    assert_eq!(cross_year.paid_days, 4);
    assert_eq!(cross_year.holidays, 8);
    assert_eq!(cross_year.vacation_pay, 800_000);
}

#[test]
fn estimates_and_rest_stay_within_supported_years_and_report_predictions() {
    for (start, end) in [("1900-01-01", "1900-01-03"), ("2100-12-30", "2100-12-31")] {
        let estimate = Estimate::new(range(start, end), Region::Federal, pay());
        assert!(estimate.predicted);
        assert!(Year::try_from(estimate.rest.start().year()).is_ok());
        assert!(Year::try_from(estimate.rest.end().year()).is_ok());
    }
    let naive = Estimate::new(range("2026-01-01", "2026-01-09"), Region::Naive, pay());
    assert_eq!(naive.paid_days, 9);
    assert_eq!(naive.holidays, 0);
    assert!(!naive.predicted);
}

#[test]
fn calendar_day_amounts_follow_monthly_workday_norms_and_transferred_workdays() {
    for (input, count, expected) in [
        ("2026-07-01", 23, -374),
        ("2026-08-03", 21, -540),
        ("2026-10-05", 22, -453),
        ("2026-10-03", 22, 1365),
        ("2026-11-04", 21, 0),
        ("2024-04-27", 21, -540),
    ] {
        assert_eq!(
            pay()
                .day_change(
                    Region::Federal.day(date(input)),
                    NonZeroU8::new(count).unwrap()
                )
                .rubles(),
            expected,
            "{input}"
        );
    }
}

#[test]
fn vacation_settings_roundtrip_and_old_documents_use_defaults() {
    let mut document = crate::test_support::document();
    document.vacation.enabled = true;
    document.vacation.average = Some("1750.25".parse().unwrap());
    let mut json = serde_json::to_value(&document).unwrap();
    assert_eq!(
        crate::model::Document::parse(&json.to_string())
            .unwrap()
            .vacation,
        document.vacation
    );
    json.as_object_mut().unwrap().remove("vacation");
    assert_eq!(
        crate::model::Document::parse(&json.to_string())
            .unwrap()
            .vacation,
        Settings::default()
    );
    json["vacation"] = serde_json::json!({"enabled": true, "salary": 0, "average": null});
    assert!(crate::model::Document::parse(&json.to_string()).is_err());
}
