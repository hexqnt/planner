use egui::{Color32, Id, Response, Ui, Widget};

use crate::{
    model::{CalendarScope, CategoryId, Document},
    text::Language,
};

const GROUP_MARKER_SIZE: egui::Vec2 = egui::vec2(12.0, 20.0);
const GROUP_MARKER_RADIUS: f32 = 4.0;

/// Тип выбора задаёт допустимые специальные варианты без динамической диспетчеризации.
pub(in crate::app) trait CalendarSelection: Copy + PartialEq + 'static {
    const SPECIALS: &'static [Self];
    fn category(self) -> Option<CategoryId>;
    fn from_category(category: CategoryId) -> Self;
    fn label(self, language: Language) -> &'static str;
}

impl CalendarSelection for CategoryId {
    const SPECIALS: &'static [Self] = &[];
    fn category(self) -> Option<CategoryId> {
        Some(self)
    }
    fn from_category(category: CategoryId) -> Self {
        category
    }
    fn label(self, _language: Language) -> &'static str {
        "—"
    }
}

impl CalendarSelection for CalendarScope {
    const SPECIALS: &'static [Self] = &[Self::All, Self::Visible];
    fn category(self) -> Option<CategoryId> {
        match self {
            Self::Category(id) => Some(id),
            Self::All | Self::Visible => None,
        }
    }
    fn from_category(category: CategoryId) -> Self {
        Self::Category(category)
    }
    fn label(self, language: Language) -> &'static str {
        match self {
            Self::All => language.text("Все календари", "All calendars"),
            Self::Visible => language.text("Только видимые", "Visible calendars only"),
            Self::Category(_) => "—",
        }
    }
}

pub(in crate::app) struct CalendarPicker<'a, S> {
    selection: &'a mut S,
    document: &'a Document,
    id: Id,
    width: f32,
}

impl<'a, S: CalendarSelection> CalendarPicker<'a, S> {
    pub(in crate::app) const fn new(selection: &'a mut S, document: &'a Document, id: Id) -> Self {
        Self {
            selection,
            document,
            id,
            width: 150.0,
        }
    }
    pub(in crate::app) const fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }
}

impl<S: CalendarSelection> Widget for CalendarPicker<'_, S> {
    fn ui(self, ui: &mut Ui) -> Response {
        let language = self.document.language;
        let before = *self.selection;
        let category = before.category().and_then(|id| self.document.category(id));
        let name = category.map_or_else(
            || before.label(language),
            |category| category.name.get(language),
        );
        let font = egui::TextStyle::Button.resolve(ui.style());
        let mut selected = egui::text::LayoutJob::default();
        if let Some(category) = category {
            let [r, g, b] = category.color;
            selected.append(
                "● ",
                0.0,
                egui::TextFormat {
                    font_id: font.clone(),
                    color: Color32::from_rgb(r, g, b),
                    ..Default::default()
                },
            );
        }
        selected.append(
            name,
            0.0,
            egui::TextFormat {
                font_id: font,
                color: ui.visuals().text_color(),
                ..Default::default()
            },
        );
        let mut response = egui::ComboBox::from_id_salt(self.id)
            .popup_style(crate::app::appearance::dropdown_style.into())
            .width(self.width)
            .truncate()
            .selected_text(selected)
            .show_ui(ui, |ui| {
                for &value in S::SPECIALS {
                    ui.selectable_value(self.selection, value, value.label(language));
                }
                for group in &self.document.groups {
                    ui.horizontal_wrapped(|ui| {
                        ui.strong(group.name.get(language));
                        for category in &group.categories {
                            let (rect, _) =
                                ui.allocate_exact_size(GROUP_MARKER_SIZE, egui::Sense::hover());
                            let [r, g, b] = category.color;
                            ui.painter().circle_filled(
                                rect.center(),
                                GROUP_MARKER_RADIUS,
                                Color32::from_rgb(r, g, b),
                            );
                        }
                    });
                    for category in &group.categories {
                        let value = S::from_category(category.id);
                        let name = category.name.get(language);
                        let response = ui.add(egui::Button::selectable(
                            *self.selection == value,
                            (super::calendar_marker(category.color), name),
                        ));
                        response.widget_info(|| {
                            egui::WidgetInfo::labeled(
                                egui::WidgetType::Button,
                                response.enabled(),
                                name,
                            )
                        });
                        if response.clicked() {
                            *self.selection = value;
                        }
                    }
                }
            })
            .response;
        if before != *self.selection {
            response.mark_changed();
        }
        response.on_hover_text(name)
    }
}
