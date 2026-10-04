use egui::{Event, Ui};

const MIN_MULTIPLIER: f32 = 1.0;
const MAX_MULTIPLIER: f32 = 3.0;
const RAMP_WEIGHT: f32 = 0.35;
const SLOW_INTERVAL: f64 = 0.25;
const FAST_INTERVAL: f64 = 0.04;

pub(super) struct WheelAcceleration {
    last: Option<(f64, bool)>,
    multiplier: f32,
}

impl Default for WheelAcceleration {
    fn default() -> Self {
        Self::new()
    }
}

impl WheelAcceleration {
    pub(super) const fn new() -> Self {
        Self {
            last: None,
            multiplier: MIN_MULTIPLIER,
        }
    }

    pub(super) fn multiplier(&mut self, ui: &Ui) -> f32 {
        if !ui.is_enabled() || !ui.rect_contains_pointer(ui.available_rect_before_wrap()) {
            *self = Self::new();
            return MIN_MULTIPLIER;
        }
        ui.input(|input| {
            if self
                .last
                .is_some_and(|(time, _)| input.time - time >= SLOW_INTERVAL)
            {
                *self = Self::new();
            }
            for event in &input.events {
                if let Event::MouseWheel {
                    delta, modifiers, ..
                } = event
                    && delta.y != 0.0
                    && !modifiers.shift
                    && !modifiers.ctrl
                    && !modifiers.command
                {
                    self.record(input.time, delta.y > 0.0);
                }
            }
        });
        self.multiplier
    }

    #[allow(clippy::cast_possible_truncation)] // Коэффициент ограничен диапазоном 1–3.
    fn record(&mut self, time: f64, direction: bool) {
        if let Some((previous, _)) = self.last.filter(|&(previous, old_direction)| {
            old_direction == direction && (0.0..SLOW_INTERVAL).contains(&(time - previous))
        }) {
            let frequency = ((SLOW_INTERVAL - (time - previous)) / (SLOW_INTERVAL - FAST_INTERVAL))
                .clamp(0.0, 1.0);
            let target =
                (frequency as f32).mul_add(MAX_MULTIPLIER - MIN_MULTIPLIER, MIN_MULTIPLIER);
            self.multiplier = RAMP_WEIGHT.mul_add(target - self.multiplier, self.multiplier);
        } else {
            self.multiplier = MIN_MULTIPLIER;
        }
        self.last = Some((time, direction));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acceleration_ramps_gradually_and_pause_or_reversal_resets_it() {
        let mut wheel = WheelAcceleration::default();
        wheel.record(0.0, false);
        assert!((wheel.multiplier - 1.0).abs() < f32::EPSILON);
        wheel.record(0.25, false);
        assert!((wheel.multiplier - 1.0).abs() < f32::EPSILON);
        wheel.record(0.395, false);
        assert!(wheel.multiplier > 1.0 && wheel.multiplier < 2.0);
        let moderate = wheel.multiplier;
        wheel.record(0.435, false);
        assert!(wheel.multiplier > moderate && wheel.multiplier < 3.0);
        for index in 1..=20 {
            wheel.record(f64::from(index).mul_add(0.02, 0.435), false);
        }
        assert!(wheel.multiplier > 2.99 && wheel.multiplier <= 3.0);
        wheel.record(0.845, true);
        assert!((wheel.multiplier - 1.0).abs() < f32::EPSILON);
        wheel.record(0.86, true);
        assert!(wheel.multiplier > 1.0 && wheel.multiplier < 2.0);
        wheel.record(1.2, true);
        assert!((wheel.multiplier - 1.0).abs() < f32::EPSILON);
    }
}
