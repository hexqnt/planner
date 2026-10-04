use std::simd::{Simd, cmp::SimdPartialOrd};

use chrono::Datelike as _;

use crate::model::DateRange;

const LANES: usize = 4;
type DateLanes = Simd<i32, LANES>;

/// Числовые границы хранятся раздельно для последовательных SIMD-загрузок; даты преобразуются только при обновлении документа.
#[derive(Default)]
pub(super) struct Intervals {
    starts: Vec<i32>,
    ends: Vec<i32>,
}

impl Intervals {
    pub(super) fn rebuild(&mut self, ranges: impl Iterator<Item = DateRange>) {
        let capacity = ranges.size_hint().0;
        self.starts.clear();
        self.starts.reserve(capacity);
        self.ends.clear();
        self.ends.reserve(capacity);
        for range in ranges {
            self.starts.push(range.start().num_days_from_ce());
            self.ends.push(range.end().num_days_from_ce());
        }
    }

    pub(super) fn for_each_matching(&self, range: DateRange, mut visit: impl FnMut(usize)) {
        let start = range.start().num_days_from_ce();
        let end = range.end().num_days_from_ce();
        let start_bound = DateLanes::splat(start);
        let end_bound = DateLanes::splat(end);
        let (starts, start_tail) = self.starts.as_chunks::<LANES>();
        let (ends, end_tail) = self.ends.as_chunks::<LANES>();
        for (block, (starts, ends)) in starts.iter().zip(ends).enumerate() {
            let mut mask = (DateLanes::from_array(*starts).simd_le(end_bound)
                & DateLanes::from_array(*ends).simd_ge(start_bound))
            .to_bitmask();
            while mask != 0 {
                visit(block * LANES + usize::try_from(mask.trailing_zeros()).expect("Four lanes"));
                mask &= mask - 1;
            }
        }
        let offset = self.starts.len() - start_tail.len();
        for (lane, (&a, &b)) in start_tail.iter().zip(end_tail).enumerate() {
            if a <= end && b >= start {
                visit(offset + lane);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_scalar_dates_in_order_including_tails_and_boundaries() {
        let dates = [
            "1900-01-01",
            "2024-02-28",
            "2024-02-29",
            "2024-03-01",
            "2100-12-31",
        ]
        .map(|date| date.parse().unwrap());
        let ranges: Vec<_> = dates
            .iter()
            .flat_map(|&a| dates.iter().map(move |&b| DateRange::between(a, b)))
            .collect();
        let mut intervals = Intervals::default();
        for len in 0..=ranges.len() {
            intervals.rebuild(ranges[..len].iter().copied());
            for &query in &ranges {
                let expected: Vec<_> = ranges[..len]
                    .iter()
                    .enumerate()
                    .filter_map(|(index, range)| range.overlaps(query).then_some(index))
                    .collect();
                let mut actual = Vec::new();
                intervals.for_each_matching(query, |index| actual.push(index));
                assert_eq!(actual, expected);
            }
        }
    }
}
