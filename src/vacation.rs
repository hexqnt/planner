//! Приближённая оценка ежегодного отпуска для российской пятидневки. Налоги и другие отсутствия не учитываются.

use std::{
    num::{NonZeroU8, NonZeroU32},
    str::FromStr,
};

use chrono::{Datelike as _, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::{
    calendar::{Day, Region},
    model::{DateRange, Year},
};

/// Положительная сумма в копейках; диапазон типа ограничивает также промежуточные вычисления.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Amount(NonZeroU32);

impl Amount {
    /// Создаёт сумму из положительного количества копеек без текстового преобразования.
    pub const fn from_cents(cents: NonZeroU32) -> Self {
        Self(cents)
    }

    pub const fn cents(self) -> i64 {
        self.0.get() as i64
    }
}

impl FromStr for Amount {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let error = "Enter a positive amount up to 42949672.95 with at most two decimal places";
        let mut whole = 0_u32;
        let mut fraction = 0_u32;
        let mut decimals = None;
        let mut digits = 0;
        for ch in value.trim().chars() {
            match ch {
                '0'..='9' => {
                    let digit = u32::from(ch) - u32::from('0');
                    if let Some(count) = &mut decimals {
                        *count += 1;
                        if *count > 2 {
                            return Err(error);
                        }
                        fraction = fraction * 10 + digit;
                    } else {
                        whole = whole
                            .checked_mul(10)
                            .and_then(|n| n.checked_add(digit))
                            .ok_or(error)?;
                        digits += 1;
                    }
                }
                '.' | ',' if decimals.is_none() && digits > 0 => decimals = Some(0),
                ' ' | '\u{a0}' | '\u{202f}' if decimals.is_none() => {}
                _ => return Err(error),
            }
        }
        if digits == 0 || decimals == Some(0) {
            return Err(error);
        }
        if decimals == Some(1) {
            fraction *= 10;
        }
        let cents = whole
            .checked_mul(100)
            .and_then(|n| n.checked_add(fraction))
            .ok_or(error)?;
        NonZeroU32::new(cents).map(Self).ok_or(error)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub enabled: bool,
    pub salary: Amount,
    pub average: Option<Amount>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: false,
            salary: Amount(NonZeroU32::new(4_000_000).expect("Positive default salary")),
            average: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pay {
    pub salary: Amount,
    pub average: Option<Amount>,
}

impl Pay {
    pub const fn daily(self) -> Rate {
        match self.average {
            Some(amount) => Rate {
                numerator: amount.cents(),
                denominator: 1,
            },
            None => Rate {
                numerator: self.salary.cents() * 10,
                denominator: 293,
            },
        }
    }

    pub const fn day_change(self, day: Day, working_days: NonZeroU8) -> Rate {
        if day.flags.is_holiday() {
            Rate {
                numerator: 0,
                denominator: 1,
            }
        } else if day.flags.is_working_day() {
            let daily = self.daily();
            // Рабочий день принадлежит этому месяцу, поэтому его норма не может быть нулевой.
            let count = working_days.get() as i64;
            Rate {
                numerator: daily.numerator * count - self.salary.cents() * daily.denominator,
                denominator: daily.denominator * count,
            }
        } else {
            self.daily()
        }
    }
}

/// Рациональная ставка сохраняет точность до округления итоговой выплаты.
#[derive(Clone, Copy, Debug)]
pub struct Rate {
    numerator: i64,
    denominator: i64,
}

impl Rate {
    pub const fn payment(self, days: u32) -> i64 {
        round_div(self.numerator * days as i64, self.denominator)
    }

    pub const fn rubles(self) -> i64 {
        round_div(self.numerator, self.denominator * 100)
    }
}

const fn round_div(numerator: i64, denominator: i64) -> i64 {
    if numerator < 0 {
        -((-numerator + denominator / 2) / denominator)
    } else {
        (numerator + denominator / 2) / denominator
    }
}

pub struct MonthEstimate {
    pub first: NaiveDate,
    pub salary: i64,
    pub loss: i64,
}

pub struct Estimate {
    pub paid_days: u32,
    pub missed_workdays: u32,
    pub holidays: u32,
    pub rest: DateRange,
    pub vacation_pay: i64,
    pub salary_loss: i64,
    pub remaining_salary: i64,
    pub predicted: bool,
    pub months: Vec<MonthEstimate>,
}

impl Estimate {
    /// # Panics
    /// Паника означает нарушение инварианта поддерживаемых дат или начала календарного месяца.
    #[must_use]
    pub fn new(range: DateRange, region: Region, pay: Pay) -> Self {
        let mut result = Self {
            paid_days: 0,
            missed_workdays: 0,
            holidays: 0,
            rest: range,
            vacation_pay: 0,
            salary_loss: 0,
            remaining_salary: 0,
            predicted: false,
            months: Vec::new(),
        };
        let mut first = range.start().with_day(1).expect("Valid month start");
        loop {
            let mut working_days = 0;
            let mut missed = 0;
            for date in first
                .iter_days()
                .take_while(|date| date.month() == first.month())
            {
                let day = region.day(date);
                result.predicted |= day.predicted;
                working_days += u32::from(day.flags.is_working_day());
                if range.contains(date) {
                    missed += u32::from(day.flags.is_working_day());
                    result.holidays += u32::from(day.flags.is_holiday());
                    result.paid_days += u32::from(!day.flags.is_holiday());
                }
            }
            let loss = if missed == 0 {
                0
            } else {
                round_div(
                    pay.salary.cents() * i64::from(missed),
                    i64::from(working_days),
                )
            };
            let salary = pay.salary.cents() - loss;
            result.salary_loss += loss;
            result.remaining_salary += salary;
            result.missed_workdays += missed;
            result.months.push(MonthEstimate {
                first,
                salary,
                loss,
            });
            if first.year() == range.end().year() && first.month() == range.end().month() {
                break;
            }
            first = first
                .checked_add_months(chrono::Months::new(1))
                .expect("Supported next month");
        }
        result.vacation_pay = pay.daily().payment(result.paid_days);
        let extend = |mut date: NaiveDate, forward: bool, predicted: &mut bool| {
            loop {
                let next = if forward {
                    date.succ_opt()
                } else {
                    date.pred_opt()
                };
                let Some(next) = next.filter(|date| Year::try_from(date.year()).is_ok()) else {
                    break;
                };
                let day = region.day(next);
                if day.flags.is_working_day() {
                    break;
                }
                *predicted |= day.predicted;
                date = next;
            }
            date
        };
        result.rest = DateRange::between(
            extend(range.start(), false, &mut result.predicted),
            extend(range.end(), true, &mut result.predicted),
        );
        result
    }

    #[must_use]
    pub const fn income_change(&self) -> i64 {
        self.vacation_pay - self.salary_loss
    }
}

#[cfg(test)]
mod tests;
