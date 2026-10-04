//! Параметры для подгонки интерфейса. Размеры и отступы заданы в логических пунктах egui, независимо от масштаба экрана.
//! Здесь собраны общие стили и параметры компоновки для ручной подгонки. Детали рисования и вычисляемые размеры остаются в модулях компонентов.
//! Изменения применяются после пересборки; значения по умолчанию сохраняют текущее оформление.

pub(super) mod style {
    use egui::{Color32, Vec2};

    /// Максимальная ширина пояснения; текст переносится по доступному месту.
    pub const HELP_MAX_WIDTH: f32 = 360.0;

    /// Запас по краям экрана для всплывающего пояснения.
    pub const HELP_SCREEN_MARGIN: f32 = 20.0;

    /// Размер основного текста.
    pub const BODY_FONT_SIZE: f32 = 15.0;

    /// Размер текста кнопок и выпадающих списков.
    pub const BUTTON_FONT_SIZE: f32 = 15.0;

    /// Размер заголовков.
    pub const HEADING_FONT_SIZE: f32 = 22.0;

    /// Насыщенность основного шрифта Inter.
    pub const FONT_WEIGHT: f32 = 400.0;

    /// Насыщенность шрифта заголовков Inter.
    pub const SEMIBOLD_WEIGHT: f32 = 650.0;

    /// Общий интервал между виджетами: горизонтальный и вертикальный.
    pub const ITEM_SPACING: Vec2 = Vec2::new(10.0, 8.0);

    /// Внутренние отступы кнопок: горизонтальный и вертикальный.
    pub const BUTTON_PADDING: Vec2 = Vec2::new(12.0, 8.0);

    /// Размер иконок действий с текстовой подписью.
    pub const ACTION_ICON_SIZE: Vec2 = Vec2::splat(18.0);

    /// Размер иконок верхней панели.
    pub const TOOLBAR_ICON_SIZE: Vec2 = Vec2::splat(22.0);

    /// Размер иконок действий внутри строки списка.
    pub const INLINE_ICON_SIZE: Vec2 = Vec2::splat(16.0);

    /// Текст на акцентном фоне.
    pub const TEXT_ON_ACCENT: Color32 = Color32::WHITE;

    /// Радиус скругления кнопок и переключателей языка.
    pub const CORNER_RADIUS: u8 = 7;

    /// Толщина рамок обычных виджетов.
    pub const BORDER_WIDTH: f32 = 1.0;

    /// Основной акцент: выделение, активные кнопки и сегодняшний день.
    pub const BLUE: Color32 = Color32::from_rgb(28, 132, 250);

    /// Цвет ошибок и выходных дней.
    pub const RED: Color32 = Color32::from_rgb(238, 69, 79);

    /// Цвет сообщений об ошибках.
    pub const ERROR: Color32 = RED;

    /// Цвет предупреждений о неполных или прогнозных данных.
    pub const WARNING: Color32 = RED;

    /// Фон панелей и окон тёмной темы.
    pub const DARK_BACKGROUND: Color32 = Color32::from_rgb(17, 21, 27);

    /// Фон панелей и окон светлой темы.
    pub const LIGHT_BACKGROUND: Color32 = Color32::from_rgb(248, 250, 253);

    /// Цвет рамок тёмной темы.
    pub const DARK_BORDER: Color32 = Color32::from_rgb(44, 53, 69);

    /// Цвет рамок светлой темы.
    pub const LIGHT_BORDER: Color32 = Color32::from_rgb(220, 227, 238);

    /// Основной текст тёмной темы.
    pub const DARK_TEXT: Color32 = Color32::from_rgb(216, 224, 241);

    /// Основной текст светлой темы.
    pub const LIGHT_TEXT: Color32 = Color32::from_rgb(27, 34, 65);
}

pub(super) mod layout {
    use egui::{Margin, Vec2};

    /// Высота верхней панели.
    pub const HEADER_HEIGHT: f32 = 72.0;

    /// Внутренние отступы верхней панели.
    pub const HEADER_MARGIN: Margin = Margin::symmetric(22, 16);

    /// Ширина области меню и выбора региона.
    pub const HEADER_LEFT_WIDTH: f32 = 240.0;

    /// Ширина области импорта, экспорта, настроек, темы и языка.
    pub const HEADER_RIGHT_WIDTH: f32 = 110.0;

    /// Ширина списка регионов.
    pub const REGION_WIDTH: f32 = 168.0;

    /// Ширина переключателя зоны просмотра.
    pub const TIMEZONE_WIDTH: f32 = 160.0;

    /// Размеры меню в логических пунктах; на маленьком экране ограничения уменьшаются до доступного места.
    pub const TIMEZONE_MENU_DEFAULT_SIZE: Vec2 = Vec2::new(300.0, 520.0);
    pub const TIMEZONE_MENU_MIN_SIZE: Vec2 = Vec2::new(280.0, 400.0);
    pub const TIMEZONE_MENU_MAX_SIZE: Vec2 = Vec2::new(520.0, 680.0);

    /// Минимальная ширина содержимого для выбора зоны в первой строке.
    pub const TIMEZONE_INLINE_MIN_WIDTH: f32 = 1050.0;

    /// Дополнительная строка зоны просмотра на узких экранах.
    pub const TIMEZONE_ROW_HEIGHT: f32 = 40.0;

    /// Размер номера года.
    pub const YEAR_FONT_SIZE: f32 = 26.0;

    /// Размер кнопок предыдущего и следующего года.
    pub const YEAR_STEP_BUTTON_SIZE: Vec2 = Vec2::new(40.0, 34.0);

    /// Размер кнопки переключения темы.
    pub const THEME_BUTTON_SIZE: Vec2 = Vec2::new(32.0, 34.0);

    /// Внутренние отступы нижней панели.
    pub const FOOTER_MARGIN: Margin = Margin::symmetric(20, 10);

    /// Ширина боковой панели.
    pub const SIDEBAR_WIDTH: f32 = 360.0;

    /// Внутренние отступы боковой панели.
    pub const SIDEBAR_MARGIN: Margin = Margin::symmetric(18, 10);

    /// Высота, резервируемая под показатель задержки кадра.
    pub const PERFORMANCE_HEIGHT: f32 = 22.0;

    /// Размер текста показателя задержки кадра.
    pub const PERFORMANCE_FONT_SIZE: f32 = 12.0;

    /// Яркость показателя задержки в тёмной теме.
    pub const PERFORMANCE_DARK_GRAY: u8 = 150;

    /// Яркость показателя задержки в светлой теме.
    pub const PERFORMANCE_LIGHT_GRAY: u8 = 110;
}

pub(super) mod dialog {
    use egui::{Margin, Vec2};

    /// Внутренние отступы диалоговых окон.
    pub const WINDOW_MARGIN: Margin = Margin::same(20);

    /// Радиус скругления диалоговых окон.
    pub const WINDOW_RADIUS: u8 = 12;

    /// Дополнительный интервал после шапки окна.
    pub const HEADER_GAP: f32 = 12.0;

    /// Размер заголовка и иконки в шапке информационного окна.
    pub const HEADING_FONT_SIZE: f32 = 26.0;

    /// Минимальный размер окна формы.
    pub const FORM_WINDOW_MIN_SIZE: Vec2 = Vec2::new(300.0, 480.0);

    /// Начальный размер окна формы, ограничиваемый размером экрана.
    pub const FORM_WINDOW_DEFAULT_SIZE: Vec2 = Vec2::new(420.0, 500.0);

    /// Запас ширины и высоты экрана при выборе начального размера окна.
    pub const FORM_SCREEN_RESERVE: Vec2 = Vec2::new(56.0, 96.0);

    /// Размер заголовка окна формы.
    pub const FORM_TITLE_FONT_SIZE: f32 = 23.0;

    /// Размер и ширина колонки иконок полей.
    pub const FIELD_ICON_SIZE: f32 = 20.0;

    /// Минимальный внутренний отступ сверху и снизу элементов формы; влияет на высоту строк.
    pub const FIELD_VERTICAL_PADDING: f32 = 10.0;

    /// Дополнительный вертикальный интервал между полями поверх общего `ITEM_SPACING.y`.
    pub const FIELD_GAP: f32 = 2.0;

    /// Размер кнопки закрытия.
    pub const CLOSE_BUTTON_SIZE: Vec2 = Vec2::splat(28.0);
}

pub(super) mod event {
    use egui::Vec2;

    /// Минимальная ширина поля названия.
    pub const TITLE_MIN_WIDTH: f32 = 160.0;

    /// Минимальная высота поля описания; оставшаяся высота заполняется автоматически.
    pub const DESCRIPTION_MIN_HEIGHT: f32 = 80.0;

    /// Ширина полей даты, включая дату окончания повторений.
    pub const DATE_WIDTH: f32 = 112.0;

    /// Ширина поля времени.
    pub const TIME_WIDTH: f32 = 76.0;

    /// Размер области подписей «С» и «По».
    pub const DATE_LABEL_SIZE: Vec2 = Vec2::new(40.0, 24.0);
}

pub(super) mod calendar {
    use egui::{Color32, Margin, Vec2};

    /// Размер ячейки дня; определяет размеры месяцев и число колонок.
    pub const DAY_CELL: Vec2 = Vec2::new(32.0, 34.0);

    /// Размер дня с дополнительной строкой стоимости отпуска.
    pub const VACATION_DAY_CELL: Vec2 = Vec2::new(58.0, 52.0);

    /// Расстояние от верха блока месяца до первой строки дней.
    pub const DAYS_TOP: f32 = 62.0;

    /// Внутренние отступы календарной сетки.
    pub const GRID_MARGIN: Margin = Margin::same(24);

    /// Горизонтальный и вертикальный интервал между месяцами.
    pub const MONTH_GAP: f32 = 24.0;

    /// Радиус кругов выделения, наведения и сегодняшнего дня.
    pub const DAY_RADIUS: u8 = 14;

    /// Размер чисел дней.
    pub const DAY_FONT_SIZE: f32 = 15.0;

    /// Размер подписей дней недели.
    pub const WEEKDAY_FONT_SIZE: f32 = 13.0;

    /// Смещение названия месяца от верхнего левого угла его блока.
    pub const MONTH_TITLE_OFFSET: Vec2 = Vec2::new(3.0, 4.0);

    /// Положение центров подписей дней недели от верха блока месяца.
    pub const WEEKDAY_TOP: f32 = 44.0;

    /// Цвет будних дней недели в тёмной теме.
    pub const DARK_MUTED: Color32 = Color32::from_rgb(127, 145, 174);

    /// Цвет будних дней недели в светлой теме.
    pub const LIGHT_MUTED: Color32 = Color32::from_rgb(120, 137, 166);

    /// Интенсивность цветной полосы события в тёмной теме.
    pub const EVENT_DARK_OPACITY: f32 = 0.30;

    /// Интенсивность цветной полосы события в светлой теме.
    pub const EVENT_LIGHT_OPACITY: f32 = 0.17;

    /// Интенсивность фона выбранного диапазона.
    pub const SELECTION_OPACITY: f32 = 0.12;

    /// Толщина рамки выбранного диапазона.
    pub const SELECTION_STROKE: f32 = 1.2;

    /// Толщина окружности сегодняшнего дня.
    pub const TODAY_STROKE: f32 = 1.3;
}

pub(super) mod sidebar {
    use egui::Vec2;

    /// Внутренние отступы кнопок в строках дерева календарей.
    pub const ROW_BUTTON_PADDING: Vec2 = Vec2::new(4.0, 2.0);

    /// Размер кнопки выбора цвета календаря.
    pub const COLOR_BUTTON_SIZE: f32 = 22.0;

    /// Дополнительный интервал перед кнопками резервной копии.
    pub const BACKUP_GAP: f32 = 12.0;

    /// Дополнительный интервал перед списком событий.
    pub const EVENTS_GAP: f32 = 20.0;

    /// Дополнительный интервал перед разделителем группы и после него.
    pub const GROUP_GAP: f32 = 4.0;

    /// Множитель стандартного отступа вложенных календарей.
    pub const TREE_INDENT_FACTOR: f32 = 2.0;

    /// Радиус скругления флажков видимости календарей.
    pub const CHECKBOX_RADIUS: u8 = 4;

    /// Толщина галочки выбранного календаря.
    pub const CHECKBOX_STROKE: f32 = 1.8;

    /// Дополнительный отступ названия от флажка видимости.
    pub const CHECKBOX_TEXT_GAP: f32 = 4.0;
}

pub(super) mod backup {
    use egui::Vec2;

    /// Начальный размер окна резервной копии.
    pub const WINDOW_SIZE: Vec2 = Vec2::new(620.0, 420.0);

    /// Максимальная высота прокручиваемого редактора JSON.
    pub const EDITOR_MAX_HEIGHT: f32 = 300.0;

    /// Желаемое число видимых строк редактора JSON.
    pub const EDITOR_ROWS: usize = 15;
}
