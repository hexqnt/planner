//! Единые источники SVG: egui кеширует растеризацию и текстуры по URI и размеру, поэтому пути не должны зависеть от места использования.

use egui::ImageSource;

pub(super) const WALLET: ImageSource<'static> = egui::include_image!("../../assets/wallet.svg");

pub(super) const MONEYBAG_PLUS: ImageSource<'static> =
    egui::include_image!("../../assets/moneybag-plus.svg");
pub(super) const MONEYBAG_MINUS: ImageSource<'static> =
    egui::include_image!("../../assets/moneybag-minus.svg");
pub(super) const MONEYBAG: ImageSource<'static> = egui::include_image!("../../assets/moneybag.svg");
pub(super) const SCALE: ImageSource<'static> = egui::include_image!("../../assets/scale.svg");

pub(super) const FLAG_RU: ImageSource<'static> = egui::include_image!("../../assets/flags/ru.svg");
pub(super) const FLAG_EN: ImageSource<'static> = egui::include_image!("../../assets/flags/us.svg");
pub(super) const SETTINGS: ImageSource<'static> = egui::include_image!("../../assets/settings.svg");
pub(super) const LANGUAGE: ImageSource<'static> = egui::include_image!("../../assets/language.svg");
pub(super) const CALCULATOR: ImageSource<'static> =
    egui::include_image!("../../assets/calculator.svg");
pub(super) const TIMEZONE: ImageSource<'static> = egui::include_image!("../../assets/timezone.svg");
pub(super) const VACATION: ImageSource<'static> =
    egui::include_image!("../../assets/pig-money.svg");
pub(super) const SEARCH: ImageSource<'static> = egui::include_image!("../../assets/search.svg");
pub(super) const INFO: ImageSource<'static> = egui::include_image!("../../assets/info-circle.svg");
pub(super) const HELP: ImageSource<'static> = egui::include_image!("../../assets/help.svg");
pub(super) const GITHUB: ImageSource<'static> =
    egui::include_image!("../../assets/tm/Octicons-mark-github.svg");
pub(super) const FILE_EXPORT: ImageSource<'static> =
    egui::include_image!("../../assets/file-export.svg");
pub(super) const FILE_IMPORT: ImageSource<'static> =
    egui::include_image!("../../assets/file-import.svg");
pub(super) const EDIT: ImageSource<'static> = egui::include_image!("../../assets/edit.svg");
pub(super) const TRASH: ImageSource<'static> = egui::include_image!("../../assets/trash.svg");
pub(super) const REPEAT: ImageSource<'static> = egui::include_image!("../../assets/repeat.svg");
pub(super) const ALARM: ImageSource<'static> = egui::include_image!("../../assets/alarm.svg");
pub(super) const USERS: ImageSource<'static> = egui::include_image!("../../assets/users.svg");
pub(super) const PAPERCLIP: ImageSource<'static> =
    egui::include_image!("../../assets/paperclip.svg");
pub(super) const MAP_PIN: ImageSource<'static> = egui::include_image!("../../assets/map-pin.svg");
pub(super) const VIDEO: ImageSource<'static> = egui::include_image!("../../assets/video.svg");
pub(super) const NOTES: ImageSource<'static> = egui::include_image!("../../assets/notes.svg");
pub(super) const CLOCK: ImageSource<'static> =
    egui::include_image!("../../assets/clock-hour-4.svg");
pub(super) const CALENDAR: ImageSource<'static> = egui::include_image!("../../assets/calendar.svg");

#[cfg(test)]
mod tests {
    use super::{
        CALCULATOR, CALENDAR, FILE_EXPORT, FILE_IMPORT, GITHUB, HELP, INFO, ImageSource, LANGUAGE,
        MONEYBAG, MONEYBAG_MINUS, MONEYBAG_PLUS, SCALE, SEARCH, SETTINGS, TIMEZONE, VACATION,
    };

    #[test]
    #[expect(
        clippy::default_trait_access,
        reason = "Тип Options не экспортируется из egui_extras."
    )]
    fn toolbar_and_github_icons_can_be_tinted_in_both_themes() {
        for source in [
            SETTINGS,
            LANGUAGE,
            CALCULATOR,
            TIMEZONE,
            GITHUB,
            INFO,
            HELP,
            FILE_EXPORT,
            FILE_IMPORT,
            CALENDAR,
            SEARCH,
            VACATION,
            MONEYBAG_PLUS,
            MONEYBAG_MINUS,
            MONEYBAG,
            SCALE,
        ] {
            let ImageSource::Bytes { bytes, .. } = source else {
                panic!("Expected an embedded SVG");
            };
            let image = egui_extras::image::load_svg_bytes(&bytes, &Default::default()).unwrap();
            assert!(image.pixels.iter().any(|pixel| pixel.a() == 255));
            assert!(
                image.pixels.iter().all(|pixel| {
                    let [r, g, b, a] = pixel.to_array();
                    r == a && g == a && b == a
                }),
                "Black SVG pixels cannot be tinted to a light theme color"
            );
        }
    }
}
