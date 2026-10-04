use std::sync::Arc;

use egui::{Color32, FontId, Galley};

use crate::{
    calendar::{Calendar, Day, Region},
    model::{DateRange, Year},
    text::Language,
    vacation::{Amount, Estimate, Pay, Settings},
};

use super::{
    Planner,
    dialogs::{Dialog, EventDraft, Heading},
    icons,
    ui_config::dialog,
    widgets,
};

pub(super) struct Vacation {
    pub open: bool,
    salary: widgets::MoneyDraft,
    average: widgets::MoneyDraft,
    manual: bool,
    pub pay: Option<Pay>,
    estimate: Option<CachedEstimate>,
    pub prices: Option<Prices>,
    pending_action: VacationAction,
}

#[derive(Default)]
enum VacationAction {
    #[default]
    None,
    Close,
    CreateEvent,
}

impl Vacation {
    pub fn new(settings: Settings) -> Self {
        let pay = Pay {
            salary: settings.salary,
            average: settings.average,
        };
        Self {
            open: false,
            salary: widgets::MoneyDraft::new(settings.salary),
            average: widgets::MoneyDraft::from_cents(
                settings
                    .average
                    .map_or_else(|| pay.daily().payment(1), Amount::cents),
            ),
            manual: settings.average.is_some(),
            pay: Some(pay),
            estimate: None,
            prices: None,
            pending_action: VacationAction::None,
        }
    }

    fn refresh_pay(&mut self) {
        self.pay = self.salary.value().and_then(|salary| {
            let average = if self.manual {
                Some(self.average.value()?)
            } else {
                None
            };
            Some(Pay { salary, average })
        });
    }

    pub fn estimate(&self) -> Option<&Estimate> {
        self.estimate.as_ref().map(|cached| &cached.value)
    }

    fn refresh(&mut self, range: Option<DateRange>, region: Region) {
        let (Some(range), Some(pay)) = (range, self.pay) else {
            self.estimate = None;
            return;
        };
        let key = (range, region, pay);
        if self
            .estimate
            .as_ref()
            .is_none_or(|cached| cached.key != key)
        {
            self.estimate = Some(CachedEstimate {
                key,
                value: Estimate::new(range, region, pay),
            });
        }
    }
}

struct CachedEstimate {
    key: (DateRange, Region, Pay),
    value: Estimate,
}

/// Подписи вычисляются при изменении исходных данных; рисование дней только заимствует готовые galley.
pub(super) struct Prices {
    key: (Year, Region, Pay, u32),
    months: [[Arc<Galley>; 3]; 12],
}

impl Prices {
    pub fn refresh(cache: &mut Option<Self>, ui: &egui::Ui, calendar: &Calendar, pay: Option<Pay>) {
        let Some(pay) = pay else {
            *cache = None;
            return;
        };
        let key = (
            calendar.year,
            calendar.region,
            pay,
            ui.ctx().pixels_per_point().to_bits(),
        );
        if cache.as_ref().is_some_and(|cached| cached.key == key) {
            return;
        }
        let months = ui.fonts_mut(|fonts| {
            let mut layout = |text: String| {
                fonts.layout_no_wrap(text, FontId::proportional(11.0), Color32::PLACEHOLDER)
            };
            let day_off = layout(format!("{:+}", pay.daily().rubles()));
            let holiday = layout("0".into());
            std::array::from_fn(|index| {
                let month = &calendar.months[index];
                let working_days = u8::try_from(
                    month
                        .days
                        .iter()
                        .filter(|day| day.flags.is_working_day())
                        .count(),
                )
                .expect("At most 31 days");
                let working = month
                    .days
                    .iter()
                    .copied()
                    .find(|day| day.flags.is_working_day())
                    .map_or_else(
                        || Arc::clone(&holiday),
                        |day| {
                            layout(format!(
                                "{:+}",
                                pay.day_change(
                                    day,
                                    std::num::NonZeroU8::new(working_days)
                                        .expect("A working day implies a positive monthly norm")
                                )
                                .rubles()
                            ))
                        },
                    );
                [working, Arc::clone(&day_off), Arc::clone(&holiday)]
            })
        });
        *cache = Some(Self { key, months });
    }

    pub fn label(&self, month: usize, day: Day) -> &Arc<Galley> {
        &self.months[month][if day.flags.is_holiday() {
            2
        } else {
            usize::from(!day.flags.is_working_day())
        }]
    }
}

fn money(cents: i64, language: Language, signed: bool) -> String {
    let sign = if cents < 0 {
        "−"
    } else if signed && cents > 0 {
        "+"
    } else {
        ""
    };
    let value = cents.abs();
    format!(
        "{sign}{}{}{:02} ₽",
        value / 100,
        language.text(",", "."),
        value % 100
    )
}

impl Planner {
    pub(super) fn close_vacation(&mut self) {
        self.vacation.salary.finish_edit(self.document.language);
        self.vacation.average.finish_edit(self.document.language);
        self.vacation.open = false;
        self.vacation.pending_action = VacationAction::None;
        if self.document.vacation.enabled {
            self.document.vacation.enabled = false;
            self.persistence.mark_changed();
        }
    }

    pub(super) fn vacation_window(&mut self, ctx: &egui::Context) {
        if !self.document.vacation.enabled {
            return;
        }
        self.vacation
            .refresh(self.selection.range(), self.document.region);
        if !self.vacation.open {
            self.vacation.open = true;
            ctx.request_repaint();
            return;
        }
        let language = self.language();
        let mut action = std::mem::take(&mut self.vacation.pending_action);
        let size = ctx.content_rect().size() - dialog::FORM_SCREEN_RESERVE;
        let default_size = dialog::FORM_WINDOW_DEFAULT_SIZE.min(size);
        Dialog::Vacation
            .window(ctx, language.text("Расчёт отпуска", "Vacation calculation"))
            .resizable(true)
            .min_size(dialog::FORM_WINDOW_MIN_SIZE)
            .max_size(size)
            .default_size(default_size)
            .default_pos(ctx.content_rect().center() - default_size * 0.5)
            .show(ctx, |ui| {
                if Heading::new(
                    language.text("Расчёт отпуска", "Vacation calculation"),
                    language,
                )
                .icon(
                    icons::CALCULATOR,
                    language.text("Калькулятор", "Calculator"),
                )
                .font_size(dialog::FORM_TITLE_FONT_SIZE)
                .help(
                    language.text("О расчёте отпуска", "About vacation calculation"),
                    language.text(
                        "Россия · пятидневка · до НДФЛ · приблизительный расчёт",
                        "Russia · five-day workweek · before tax · estimate",
                    ),
                )
                .show(ui)
                {
                    action = VacationAction::Close;
                }
                egui::Panel::bottom("vacation_footer")
                    .frame(egui::Frame::NONE)
                    .show_separator_line(false)
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            if self.vacation.estimate().is_some()
                                && widgets::labelled_action(
                                    widgets::primary_button(
                                        ui,
                                        language.text("Создать событие", "Create event"),
                                    ),
                                    language
                                        .text("Создать событие «Отпуск»", "Create vacation event"),
                                )
                                .clicked()
                            {
                                action = VacationAction::CreateEvent;
                            }
                            if ui.button(language.text("Закрыть", "Close")).clicked() {
                                action = VacationAction::Close;
                            }
                        });
                    });
                egui::ScrollArea::vertical()
                    .id_salt("vacation_details")
                    .max_height(ui.available_height())
                    .auto_shrink([false, false])
                    .show(ui, |ui| self.vacation_contents(ui));
            });
        if ctx.will_discard() {
            self.vacation.pending_action = action;
            return;
        }
        if !matches!(action, VacationAction::None) {
            self.close_vacation();
            ctx.request_repaint();
        }
        if matches!(action, VacationAction::CreateEvent) {
            self.new_vacation_event();
        }
    }

    fn vacation_contents(&mut self, ui: &mut egui::Ui) {
        let language = self.language();
        widgets::FormRow::new(icons::CALENDAR, language.text("Даты отпуска", "Vacation dates")).show(ui, |ui| {
            widgets::hint(ui.weak(language.text("Даты отпуска", "Vacation dates")), language.text("Выделите даты отпуска на календаре: перетаскиванием или Shift + кликом. Под датой — изменение дохода при включении этого дня в отпуск. Включённые выходные тоже расходуют дни отпуска, праздничные дни исключаются. Потеря зарплаты зависит от нормы рабочих дней каждого месяца.", "Select vacation dates on the calendar by dragging or Shift-clicking. Each date shows the income change if that day is included in leave. Included weekends also use leave days; public holidays are excluded. Salary reduction depends on each month's workday norm."));
            if let Some(range) = self.selection.range() {
                widgets::date_range(ui, range, language);
            } else {
                ui.label(
                    language.text("Выделите даты на календаре", "Select dates on the calendar"),
                );
            }
        });
        self.vacation_inputs(ui);
        if self.document.region == Region::Naive {
            widgets::warning_label(ui, language.text("Календарь Сб/Вс не учитывает праздники и переносы. Для расчёта выберите производственный календарь.", "The Sat/Sun calendar excludes holidays and transfers. Select a production calendar for the estimate."));
        }
        self.vacation
            .refresh(self.selection.range(), self.document.region);
        estimate_metrics(ui, self.vacation.estimate(), self.vacation.pay, language);
        if self.vacation.pay.is_none() {
            return;
        }
        let Some(estimate) = self.vacation.estimate() else {
            return;
        };
        ui.vertical(|ui| {
            ui.label(format!(
                "{}: {} · {}: {} · {}: {}",
                language.text("Дней отпуска", "Leave days"),
                estimate.paid_days,
                language.text("Пропущено рабочих", "Workdays missed"),
                estimate.missed_workdays,
                language.text("Праздников", "Holidays"),
                estimate.holidays
            ));
            ui.label(format!(
                "{}: {} ({} {})",
                language.text("Всего отдыха", "Total time off"),
                estimate.rest,
                estimate.rest.days(),
                language.text("дн.", "days")
            ));
        });
        if estimate.months.len() > 1 {
            ui.collapsing(language.text("По месяцам", "By month"), |ui| {
                for month in &estimate.months {
                    ui.label(format!(
                        "{} · {}: {} · {}: {}",
                        month.first.format("%m.%Y"),
                        language.text("зарплата", "salary"),
                        money(month.salary, language, false),
                        language.text("потеря", "reduction"),
                        money(month.loss, language, false)
                    ));
                }
            });
        }
        if estimate.predicted {
            widgets::warning_label(
                ui,
                language.text(
                    "В расчёте есть прогнозные календарные данные.",
                    "The estimate includes predicted calendar data.",
                ),
            );
        }
    }

    fn vacation_inputs(&mut self, ui: &mut egui::Ui) {
        let language = self.language();
        let mut changed = false;
        widgets::FormRow::new(icons::WALLET, language.text("Оклад", "Salary")).show(ui, |ui| {
            ui.horizontal(|ui| {
                let label = widgets::hint(
                    ui.weak(language.text("Оклад, ₽/мес", "Monthly salary, RUB")),
                    language.text(
                        "Оценка по окладу использует оклад / 29,3 и предполагает постоянный заработок и полностью отработанный расчётный период. Известную ставку можно взять из расчёта бухгалтерии.",
                        "The salary-based estimate uses salary / 29.3 and assumes constant earnings and a fully worked reference period. A known daily rate can be taken from payroll.",
                    ),
                );
                changed |= ui
                    .add(widgets::MoneyInput::new(
                        &mut self.vacation.salary,
                        Dialog::Vacation.id().with("salary"),
                        language.text("Оклад, ₽/мес", "Monthly salary, RUB"),
                        language,
                    ))
                    .labelled_by(label.id)
                    .changed();
            });
        });
        changed |= ui
            .checkbox(
                &mut self.vacation.manual,
                language.text(
                    "Указать средний дневной заработок",
                    "Enter average daily earnings",
                ),
            )
            .changed();
        if self.vacation.manual {
            let label =
                ui.weak(language.text("Средний заработок, ₽/день", "Average earnings, RUB/day"));
            changed |= ui
                .add(widgets::MoneyInput::new(
                    &mut self.vacation.average,
                    Dialog::Vacation.id().with("average"),
                    language.text("Средний заработок, ₽/день", "Average earnings, RUB/day"),
                    language,
                ))
                .labelled_by(label.id)
                .changed();
        }
        if changed {
            self.vacation.refresh_pay();
            if let Some(pay) = self.vacation.pay {
                if !self.vacation.manual {
                    self.vacation.average = widgets::MoneyDraft::from_cents(pay.daily().payment(1));
                }
                self.document.vacation.salary = pay.salary;
                self.document.vacation.average = pay.average;
                self.persistence.mark_changed();
            }
        }
    }

    fn new_vacation_event(&mut self) {
        let Some(range) = self.selection.range() else {
            return;
        };
        // Выбор по имени работает и для старых документов без специального типа календаря.
        let vacation_name = self.language().text("Отпуск", "Vacation");
        let category = self
            .document
            .visible_categories()
            .find(|category| category.name.get(self.language()) == vacation_name)
            .map(|category| category.id);
        if let Some(category) = category {
            self.event_draft = Some(EventDraft::new(range, category));
        } else {
            self.new_event();
        }
        if let Some(draft) = &mut self.event_draft {
            draft.set_default_timezone(self.document.display_timezone);
            draft.set_title(vacation_name);
        }
    }
}

fn estimate_metrics(
    ui: &mut egui::Ui,
    estimate: Option<&Estimate>,
    pay: Option<Pay>,
    language: Language,
) {
    let amounts = estimate
        .map(|estimate| {
            [
                estimate.vacation_pay,
                estimate.salary_loss,
                estimate.remaining_salary,
                estimate.income_change(),
            ]
        })
        .or_else(|| pay.map(|pay| [0, 0, pay.salary.cents(), 0]))
        .map_or([None; 4], |amounts| amounts.map(Some));
    egui::Grid::new("vacation_metrics")
        .num_columns(3)
        .min_col_width(0.0)
        .max_col_width(
            2.0_f32.mul_add(
                -ui.spacing().item_spacing.x,
                ui.available_width() - dialog::FIELD_ICON_SIZE,
            ) / 2.0,
        )
        .show(ui, |ui| {
            for ((icon, label, signed), cents) in [
                (icons::MONEYBAG_PLUS, language.text("Отпускные", "Vacation pay"), false),
                (icons::MONEYBAG_MINUS, language.text("Потеря зарплаты", "Salary reduction"), false),
                (icons::MONEYBAG, language.text("Оставшаяся зарплата", "Remaining salary"), false),
                (icons::SCALE, language.text("Изменение дохода", "Income change"), true),
            ]
            .into_iter()
            .zip(amounts)
            {
                ui.add(
                    egui::Image::new(icon)
                        .fit_to_exact_size(egui::Vec2::splat(dialog::FIELD_ICON_SIZE))
                        .tint(ui.visuals().weak_text_color()),
                )
                .on_hover_text(label);
                let response = ui.weak(label);
                if signed {
                    widgets::hint(response, language.text("Изменение дохода сравнивается с работой без отпуска; это не график выплаты денег. Оставшаяся зарплата показана за все затронутые месяцы; другие отсутствия и переменные выплаты не учитываются.", "Income change compares against working without leave; it is not a payment schedule. Remaining salary covers all affected months; other absences and variable earnings are excluded."));
                }
                ui.label(
                    cents.map_or_else(|| "—".into(), |cents| money(cents, language, signed)),
                );
                ui.end_row();
            }
        });
}
