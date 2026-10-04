use chrono::{Datelike as _, NaiveDate, NaiveTime};
use nucleo_matcher::{
    Matcher, Utf32String,
    pattern::{Atom, AtomKind, CaseMatching, Normalization},
};

use crate::model::{Document, Event, EventLink, Year};
use crate::text::Language;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueryError {
    Date,
    Dates,
    Time,
    Times,
}

impl QueryError {
    pub const fn message(self, language: Language) -> &'static str {
        match self {
            Self::Date => language.text(
                "Дата: ГГГГ-ММ-ДД или ДД.ММ.ГГГГ, годы 1900–2100",
                "Date: YYYY-MM-DD or DD.MM.YYYY, years 1900–2100",
            ),
            Self::Dates => language.text(
                "Для диапазона дат используйте фильтры",
                "Use filters for a date range",
            ),
            Self::Time => language.text("Время: ЧЧ:ММ или ЧЧ:ММ:СС", "Time: HH:MM or HH:MM:SS"),
            Self::Times => language.text("Введите одно время начала", "Enter one start time"),
        }
    }
}

#[derive(Clone, Copy)]
pub(super) enum Field {
    Title,
    Location,
    Notes,
    Participant(usize),
    Link,
    Attachment(usize),
    Calendar,
    Group,
}

impl Field {
    pub fn get<'a>(self, event: &'a Event, document: &'a Document) -> &'a str {
        match self {
            Self::Title => event.title.get(),
            Self::Location => &event.location,
            Self::Notes => &event.notes,
            Self::Participant(index) => event.details.participants[index].as_str(),
            Self::Link => event.link.as_ref().map_or("", EventLink::as_str),
            Self::Attachment(index) => event.details.attachments[index].as_str(),
            Self::Calendar => document
                .category(event.category)
                .map_or("", |category| category.name.get(document.language)),
            Self::Group => document
                .groups
                .iter()
                .find(|group| {
                    group
                        .categories
                        .iter()
                        .any(|category| category.id == event.category)
                })
                .map_or("", |group| group.name.get(document.language)),
        }
    }

    const fn weight(self) -> u32 {
        match self {
            Self::Title => 8,
            Self::Location | Self::Participant(_) => 4,
            Self::Calendar | Self::Group => 3,
            Self::Link | Self::Attachment(_) => 2,
            Self::Notes => 1,
        }
    }
}

pub(super) struct IndexedField {
    pub source: Field,
    pub text: Utf32String,
}

pub(super) struct IndexedText {
    pub title: Utf32String,
    pub fields: Vec<IndexedField>,
}

impl IndexedText {
    pub fn new(event: &Event, document: &Document) -> Self {
        let fields = [
            Field::Location,
            Field::Notes,
            Field::Link,
            Field::Calendar,
            Field::Group,
        ]
        .into_iter()
        .chain((0..event.details.participants.len()).map(Field::Participant))
        .chain((0..event.details.attachments.len()).map(Field::Attachment))
        .filter_map(|source| {
            let text = source.get(event, document);
            (!text.is_empty()).then(|| IndexedField {
                source,
                text: Utf32String::from(text),
            })
        })
        .collect();
        Self {
            title: Utf32String::from(event.title.get()),
            fields,
        }
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Rank {
    title: u32,
    quality: u32,
    weight: u32,
    score: u64,
}

struct Token {
    fuzzy: Atom,
    literal: [Atom; 3],
}

impl Token {
    fn new(text: &str) -> Self {
        let atom = |kind| {
            Atom::new(
                text,
                CaseMatching::Ignore,
                Normalization::Smart,
                kind,
                false,
            )
        };
        Self {
            fuzzy: atom(AtomKind::Fuzzy),
            literal: [AtomKind::Exact, AtomKind::Prefix, AtomKind::Substring].map(atom),
        }
    }

    fn score(&self, source: Field, text: &Utf32String, matcher: &mut Matcher) -> Option<Rank> {
        let text = text.slice(..);
        let score = self.fuzzy.score(text, matcher)?;
        let quality = self
            .literal
            .iter()
            .zip([3, 2, 1])
            .find_map(|(atom, quality)| atom.score(text, matcher).map(|_| quality))
            .unwrap_or(0);
        Some(Rank {
            title: if matches!(source, Field::Title) {
                quality
            } else {
                0
            },
            quality,
            weight: source.weight(),
            score: u64::from(score),
        })
    }
}

#[derive(Default)]
pub struct SearchQuery {
    tokens: Vec<Token>,
    pub(super) date: Option<NaiveDate>,
    pub(super) time: Option<NaiveTime>,
}

impl SearchQuery {
    /// Дата и время разбираются строго; остальные слова буквально сопоставляются независимо по любым полям.
    pub fn parse(text: &str) -> Result<Self, QueryError> {
        let mut query = Self::default();
        for word in text.split_whitespace() {
            if word.len() == 10
                && word.as_bytes()[..2].iter().all(u8::is_ascii_digit)
                && (word.as_bytes().get(4) == Some(&b'-') || word.as_bytes().get(2) == Some(&b'.'))
            {
                let date = NaiveDate::parse_from_str(word, "%Y-%m-%d")
                    .or_else(|_| NaiveDate::parse_from_str(word, "%d.%m.%Y"))
                    .ok()
                    .filter(|date| Year::try_from(date.year()).is_ok())
                    .ok_or(QueryError::Date)?;
                if query.date.replace(date).is_some() {
                    return Err(QueryError::Dates);
                }
            } else if matches!(word.len(), 5 | 8)
                && word.as_bytes()[..2].iter().all(u8::is_ascii_digit)
                && word.as_bytes().get(2) == Some(&b':')
            {
                let time = NaiveTime::parse_from_str(word, "%H:%M")
                    .or_else(|_| NaiveTime::parse_from_str(word, "%H:%M:%S"))
                    .map_err(|_| QueryError::Time)?;
                if query.time.replace(time).is_some() {
                    return Err(QueryError::Times);
                }
            } else {
                query.tokens.push(Token::new(word));
            }
        }
        Ok(query)
    }

    pub(super) fn score(
        &self,
        record: &IndexedText,
        matcher: &mut Matcher,
    ) -> Option<(Rank, Option<usize>)> {
        let mut rank = Rank::default();
        let mut snippet = None;
        let mut snippet_rank = Rank::default();
        for token in &self.tokens {
            let title = token
                .score(Field::Title, &record.title, matcher)
                .map(|rank| (None, rank));
            let (field, best) = record
                .fields
                .iter()
                .enumerate()
                .filter_map(|(index, field)| {
                    token
                        .score(field.source, &field.text, matcher)
                        .map(|score| (Some(index), score))
                })
                .chain(title)
                .max_by_key(|(_, score)| *score)?;
            rank.title += best.title;
            rank.quality += best.quality;
            rank.weight += best.weight;
            rank.score += best.score;
            if field.is_some() && (snippet.is_none() || best > snippet_rank) {
                snippet = field;
                snippet_rank = best;
            }
        }
        Some((rank, snippet))
    }

    pub(super) fn highlight(
        &self,
        text: &Utf32String,
        matcher: &mut Matcher,
        indices: &mut Vec<u32>,
    ) {
        for token in &self.tokens {
            token.fuzzy.indices(text.slice(..), matcher, indices);
        }
        indices.sort_unstable();
        indices.dedup();
    }
}
