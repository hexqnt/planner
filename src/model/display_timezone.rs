use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

/// Зона просмотра не меняет исходную зону события и правила его повторений.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisplayTimeZone {
    #[default]
    System,
    Utc,
    Named(Tz),
}

impl DisplayTimeZone {
    #[must_use]
    pub fn wall_time(self, instant: DateTime<Utc>) -> NaiveDateTime {
        match self {
            Self::System => instant.with_timezone(&Local).naive_local(),
            Self::Utc => instant.naive_utc(),
            Self::Named(zone) => instant.with_timezone(&zone).naive_local(),
        }
    }

    #[must_use]
    pub fn today(self) -> NaiveDate {
        self.wall_time(Utc::now()).date()
    }
}

/// Недавние IANA-зоны хранятся без повторов, самые новые идут первыми.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "Vec<Tz>")]
pub struct RecentTimeZones(Vec<Tz>);

impl RecentTimeZones {
    pub fn remember(&mut self, zone: Tz) {
        self.0.retain(|&previous| previous != zone);
        self.0.insert(0, zone);
        self.0.truncate(5);
    }

    pub fn iter(&self) -> impl Iterator<Item = Tz> + '_ {
        self.0.iter().copied()
    }
}

impl From<Vec<Tz>> for RecentTimeZones {
    fn from(zones: Vec<Tz>) -> Self {
        let mut recent = Self::default();
        for zone in zones.into_iter().rev() {
            recent.remember(zone);
        }
        recent
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn historical_wall_clocks_use_the_selected_zone() {
        let instant = "2026-12-31T23:30:00Z".parse().unwrap();
        assert_eq!(
            DisplayTimeZone::Utc.wall_time(instant).to_string(),
            "2026-12-31 23:30:00"
        );
        assert_eq!(
            DisplayTimeZone::Named(chrono_tz::Europe::Moscow)
                .wall_time(instant)
                .to_string(),
            "2027-01-01 02:30:00"
        );
        assert_eq!(
            DisplayTimeZone::System.wall_time(instant),
            instant.with_timezone(&Local).naive_local()
        );
        let zone = DisplayTimeZone::Named(chrono_tz::America::New_York);
        for (instant, expected) in [
            ("2026-03-07T14:00:00Z", "2026-03-07 09:00:00"),
            ("2026-03-09T13:00:00Z", "2026-03-09 09:00:00"),
        ] {
            assert_eq!(
                zone.wall_time(instant.parse().unwrap()).to_string(),
                expected
            );
        }
    }

    #[test]
    fn recent_zones_move_to_front_and_evict_the_oldest_choice() {
        use chrono_tz::{
            America::New_York,
            Asia::{Dubai, Tokyo},
            Europe::{Berlin, Moscow},
            Pacific::Auckland,
        };
        let mut recent = RecentTimeZones::default();
        assert_eq!(recent.iter().count(), 0);
        for zone in [Moscow, Berlin, Tokyo, New_York, Dubai] {
            recent.remember(zone);
        }
        assert_eq!(
            recent.iter().collect::<Vec<_>>(),
            [Dubai, New_York, Tokyo, Berlin, Moscow]
        );
        recent.remember(Tokyo);
        assert_eq!(
            recent.iter().collect::<Vec<_>>(),
            [Tokyo, Dubai, New_York, Berlin, Moscow]
        );
        recent.remember(Auckland);
        assert_eq!(
            recent.iter().collect::<Vec<_>>(),
            [Auckland, Tokyo, Dubai, New_York, Berlin]
        );
    }

    #[test]
    fn deserialized_recent_zones_keep_first_occurrences_and_at_most_five_choices() {
        let recent: RecentTimeZones = serde_json::from_value(serde_json::json!([
            "Europe/Moscow",
            "Europe/Berlin",
            "Europe/Moscow",
            "Asia/Tokyo",
            "America/New_York",
            "Asia/Dubai",
            "Pacific/Auckland",
        ]))
        .unwrap();
        assert_eq!(
            recent.iter().collect::<Vec<_>>(),
            [
                chrono_tz::Europe::Moscow,
                chrono_tz::Europe::Berlin,
                chrono_tz::Asia::Tokyo,
                chrono_tz::America::New_York,
                chrono_tz::Asia::Dubai,
            ]
        );
        assert_eq!(
            serde_json::from_value::<RecentTimeZones>(serde_json::to_value(&recent).unwrap())
                .unwrap(),
            recent
        );
        assert!(
            serde_json::from_value::<RecentTimeZones>(serde_json::json!(["Unknown/Zone"])).is_err()
        );
    }
}
