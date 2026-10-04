use serde::{Deserialize, Serialize};

use crate::model::InputError;

use super::EventLink;

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct EventDetails {
    pub participants: Vec<Email>,
    pub attachments: Vec<EventLink>,
    pub reminders: Vec<Reminder>,
}

/// Базовый ASCII-адрес: dot-atom и доменное имя, без display name и литералов IP.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct Email(String);

impl Email {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn parse_address(value: &str) -> Result<&str, InputError> {
        let value = value.trim();
        let (local, domain) = value.split_once('@').ok_or(InputError::Participant)?;
        let valid_local = !local.is_empty()
            && local.len() <= 64
            && !local.starts_with('.')
            && !local.ends_with('.')
            && !local.contains("..")
            && local.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || b".!#$%&'*+-/=?^_`{|}~".contains(&byte)
            });
        let valid_domain = domain.contains('.')
            && domain.len() <= 253
            && domain.split('.').all(|label| {
                !label.is_empty()
                    && label.len() <= 63
                    && !label.starts_with('-')
                    && !label.ends_with('-')
                    && label
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
            });
        if value.len() > 254 || !valid_local || !valid_domain {
            return Err(InputError::Participant);
        }
        Ok(value)
    }
}

impl TryFrom<String> for Email {
    type Error = InputError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let address = Self::parse_address(&value)?;
        if address.len() == value.len() {
            Ok(Self(value))
        } else {
            Ok(Self(address.into()))
        }
    }
}

impl std::str::FromStr for Email {
    type Err = InputError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse_address(value).map(|value| Self(value.into()))
    }
}

/// Напоминание относительно начала; ноль означает начало события.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u32", into = "u32")]
pub struct Reminder(u32);

impl Reminder {
    pub const MAX_MINUTES: u32 = 28 * 24 * 60;
    pub const DEFAULT: Self = Self(15);

    pub const fn minutes(self) -> u32 {
        self.0
    }
}

impl TryFrom<u32> for Reminder {
    type Error = InputError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        if value <= Self::MAX_MINUTES {
            Ok(Self(value))
        } else {
            Err(InputError::Reminder)
        }
    }
}

impl From<Reminder> for u32 {
    fn from(value: Reminder) -> Self {
        value.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emails_preserve_trimmed_addresses_on_every_input_path() {
        for value in ["a+b@example.com", " first.last@EXAMPLE.com "] {
            for email in [
                value.parse::<Email>().unwrap(),
                Email::try_from(value.to_owned()).unwrap(),
                serde_json::from_value::<Email>(serde_json::json!(value)).unwrap(),
            ] {
                assert_eq!(email.as_str(), value.trim());
                assert_eq!(serde_json::to_value(email).unwrap(), value.trim());
            }
        }
    }

    #[test]
    fn emails_reject_invalid_syntax_on_every_input_path() {
        for value in [
            "",
            "missing-at",
            "a@@example.com",
            ".a@example.com",
            "a.@example.com",
            "a..b@example.com",
            "a@-example.com",
            "a@example-.com",
            "a@example..com",
            "a@example.com.",
            "a b@example.com",
            "Name <a@example.com>",
            "a@localhost",
            "я@example.com",
        ] {
            assert_eq!(
                value.parse::<Email>(),
                Err(InputError::Participant),
                "{value}"
            );
            assert_eq!(
                Email::try_from(value.to_owned()),
                Err(InputError::Participant),
                "{value}"
            );
            assert!(
                serde_json::from_value::<Email>(serde_json::json!(value)).is_err(),
                "{value}"
            );
        }
    }

    #[test]
    fn email_length_limits_are_inclusive_for_local_parts_labels_and_whole_addresses() {
        let label = "d".repeat(63);
        let max_domain = format!("{label}.{label}.{label}.{}", "d".repeat(61));
        let max_address_domain = format!("{label}.{label}.{}", "d".repeat(61));
        for (value, valid) in [
            (format!("{}@example.com", "a".repeat(64)), true),
            (format!("{}@example.com", "a".repeat(65)), false),
            (format!("a@{label}.com"), true),
            (format!("a@{label}d.com"), false),
            (format!("{}@{max_address_domain}", "a".repeat(64)), true),
            (format!("{}@{max_address_domain}d", "a".repeat(64)), false),
            (format!("a@{max_domain}"), false),
        ] {
            assert_eq!(
                value.parse::<Email>().is_ok(),
                valid,
                "{} bytes: {value}",
                value.len()
            );
            assert_eq!(
                serde_json::from_value::<Email>(serde_json::json!(value)).is_ok(),
                valid,
                "{value}"
            );
        }
    }

    #[test]
    fn reminders_accept_both_limits_and_reject_values_above_the_maximum() {
        for value in [0, Reminder::DEFAULT.minutes(), Reminder::MAX_MINUTES] {
            let reminder = Reminder::try_from(value).unwrap();
            assert_eq!(reminder.minutes(), value);
            assert_eq!(serde_json::to_value(reminder).unwrap(), value);
            assert_eq!(
                serde_json::from_value::<Reminder>(serde_json::json!(value)).unwrap(),
                reminder
            );
        }
        for value in [Reminder::MAX_MINUTES + 1, u32::MAX] {
            assert_eq!(Reminder::try_from(value), Err(InputError::Reminder));
            assert!(serde_json::from_value::<Reminder>(serde_json::json!(value)).is_err());
        }
        for value in [serde_json::json!(-1), serde_json::json!(1.5)] {
            assert!(serde_json::from_value::<Reminder>(value).is_err());
        }
    }
}
