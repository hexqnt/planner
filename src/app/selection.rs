use chrono::NaiveDate;

use crate::model::DateRange;

/// Опорная дата и интервал существуют вместе; перетаскивание невозможно без выделения.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub(super) enum Selection {
    #[default]
    Empty,
    Selected(Span),
    Dragging(Span),
}

impl Selection {
    pub const fn range(self) -> Option<DateRange> {
        match self {
            Self::Empty => None,
            Self::Selected(span) | Self::Dragging(span) => Some(span.range),
        }
    }

    pub const fn is_dragging(self) -> bool {
        matches!(self, Self::Dragging(_))
    }

    pub const fn clear(&mut self) {
        *self = Self::Empty;
    }

    pub const fn set_range(&mut self, range: DateRange) {
        *self = Self::Selected(Span {
            range,
            anchor: range.start(),
        });
    }

    pub fn click(&mut self, date: NaiveDate, extend: bool) {
        let anchor = match *self {
            Self::Selected(span) | Self::Dragging(span) if extend => span.anchor,
            _ => date,
        };
        *self = Self::Selected(Span {
            range: DateRange::between(anchor, date),
            anchor,
        });
    }

    pub fn start_drag(&mut self, date: NaiveDate) {
        *self = Self::Dragging(Span {
            range: DateRange::between(date, date),
            anchor: date,
        });
    }

    pub fn drag_to(&mut self, date: NaiveDate) {
        if let Self::Dragging(span) = self {
            span.range = DateRange::between(span.anchor, date);
        }
    }

    pub const fn finish_drag(&mut self) {
        if let Self::Dragging(span) = *self {
            *self = Self::Selected(span);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Span {
    range: DateRange,
    anchor: NaiveDate,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 1, day).unwrap()
    }

    #[test]
    fn shift_click_preserves_anchor_when_extending_in_both_directions() {
        let mut selection = Selection::default();
        selection.click(date(15), true);
        assert_eq!(selection.range().unwrap().days(), 1);
        selection.click(date(10), true);
        assert_eq!(
            selection.range().unwrap(),
            DateRange::between(date(10), date(15))
        );
        selection.click(date(20), true);
        assert_eq!(
            selection.range().unwrap(),
            DateRange::between(date(15), date(20))
        );
        selection.click(date(25), false);
        selection.click(date(30), true);
        assert_eq!(
            selection.range().unwrap(),
            DateRange::between(date(25), date(30))
        );
    }

    #[test]
    fn finishing_or_clearing_drag_prevents_later_pointer_moves() {
        let mut selection = Selection::default();
        selection.drag_to(date(3));
        assert_eq!(selection.range(), None);
        selection.start_drag(date(10));
        selection.drag_to(date(5));
        assert!(selection.is_dragging());
        selection.finish_drag();
        selection.drag_to(date(20));
        assert_eq!(
            selection.range().unwrap(),
            DateRange::between(date(5), date(10))
        );
        selection.click(date(15), true);
        assert_eq!(
            selection.range().unwrap(),
            DateRange::between(date(10), date(15))
        );
        selection.start_drag(date(1));
        selection.clear();
        selection.drag_to(date(20));
        assert_eq!(selection, Selection::Empty);
    }
}
