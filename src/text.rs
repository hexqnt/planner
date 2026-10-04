use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    #[default]
    Russian,
    English,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LanguageMode {
    #[default]
    Manual,
    Auto,
}

impl Language {
    #[must_use]
    pub fn from_locale(locale: &str) -> Self {
        let primary = locale.split(['-', '_', '.', '@']).next().unwrap_or("");
        if primary.eq_ignore_ascii_case("ru") {
            Self::Russian
        } else {
            Self::English
        }
    }

    #[must_use]
    pub fn system() -> Self {
        sys_locale::get_locale()
            .as_deref()
            .map_or(Self::English, Self::from_locale)
    }

    #[must_use]
    pub const fn text<'a>(self, ru: &'a str, en: &'a str) -> &'a str {
        match self {
            Self::Russian => ru,
            Self::English => en,
        }
    }

    #[must_use]
    pub const fn months(self) -> [&'static str; 12] {
        match self {
            Self::Russian => [
                "Январь",
                "Февраль",
                "Март",
                "Апрель",
                "Май",
                "Июнь",
                "Июль",
                "Август",
                "Сентябрь",
                "Октябрь",
                "Ноябрь",
                "Декабрь",
            ],
            Self::English => [
                "January",
                "February",
                "March",
                "April",
                "May",
                "June",
                "July",
                "August",
                "September",
                "October",
                "November",
                "December",
            ],
        }
    }

    #[must_use]
    pub const fn weekdays(self) -> [&'static str; 7] {
        match self {
            Self::Russian => ["Пн", "Вт", "Ср", "Чт", "Пт", "Сб", "Вс"],
            Self::English => ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameError {
    Empty,
}

impl std::fmt::Display for NameError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => formatter.write_str("Name is required"),
        }
    }
}

impl std::error::Error for NameError {}

#[derive(Serialize, Deserialize)]
#[serde(try_from = "NameInput")]
pub struct Name {
    ru: String,
    en: String,
}

impl Name {
    pub fn new(ru: &str, en: &str) -> Result<Self, NameError> {
        let ru = Self::parse_part(ru)?;
        let en = Self::parse_part(en)?;
        Ok(Self {
            ru: ru.into(),
            en: en.into(),
        })
    }

    pub fn custom(name: &str) -> Result<Self, NameError> {
        Self::new(name, name)
    }

    pub fn get(&self, language: Language) -> &str {
        language.text(&self.ru, &self.en)
    }

    fn parse_part(value: &str) -> Result<&str, NameError> {
        if value.trim().is_empty() {
            Err(NameError::Empty)
        } else {
            Ok(value)
        }
    }
}

impl TryFrom<NameInput> for Name {
    type Error = NameError;

    fn try_from(input: NameInput) -> Result<Self, Self::Error> {
        Self::parse_part(&input.ru)?;
        Self::parse_part(&input.en)?;
        Ok(Self {
            ru: input.ru,
            en: input.en,
        })
    }
}
#[derive(Deserialize)]
struct NameInput {
    ru: String,
    en: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_selects_russian_or_falls_back_to_english() {
        for locale in ["ru", "ru-RU", "ru_RU.UTF-8", "RU-ru", "ru_RU@variant"] {
            assert!(Language::from_locale(locale) == Language::Russian);
        }
        for locale in ["en-US", "en_GB.UTF-8", "de-DE", "", "C", "russian", "uk-UA"] {
            assert!(Language::from_locale(locale) == Language::English);
        }
    }

    #[test]
    fn names_reject_empty_translations_on_all_input_paths() {
        for empty in ["", " ", "\n\t"] {
            for (ru, en) in [(empty, "Calendar"), ("Календарь", empty)] {
                assert!(matches!(Name::new(ru, en), Err(NameError::Empty)));
                assert!(
                    serde_json::from_value::<Name>(serde_json::json!({"ru": ru, "en": en}))
                        .is_err()
                );
            }
            assert!(matches!(Name::custom(empty), Err(NameError::Empty)));
        }
        assert_eq!(NameError::Empty.to_string(), "Name is required");
    }

    #[test]
    fn localized_and_custom_names_preserve_text_when_restored() {
        for name in [
            Name::new(" Календарь ", " Calendar ").unwrap(),
            Name::custom("Мой календарь").unwrap(),
        ] {
            let restored: Name =
                serde_json::from_value(serde_json::to_value(&name).unwrap()).unwrap();
            for language in [Language::Russian, Language::English] {
                assert_eq!(restored.get(language), name.get(language));
            }
        }
    }
}
