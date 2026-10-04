use egui::{Event, Ui};

/// Остаток движения сглаживается по времени, независимо от частоты кадров.
#[derive(Default)]
pub(super) struct SmoothScroll {
    remaining: f32,
    suppress_tail: bool,
}

impl SmoothScroll {
    pub(super) const fn cancel(&mut self) {
        self.remaining = 0.0;
        self.suppress_tail = true;
    }

    pub(super) fn offset(&mut self, ui: &Ui, offset: f32, max_offset: f32, multiplier: f32) -> f32 {
        let hovered = ui.is_enabled() && ui.rect_contains_pointer(ui.available_rect_before_wrap());
        let (delta, dt) = ui.input_mut(|input| {
            if input.pointer.any_pressed() {
                self.cancel();
            }
            let fresh_wheel = input.events.iter().any(|event| {
                matches!(event, Event::MouseWheel { delta, modifiers, .. }
                    if delta.y != 0.0 && !modifiers.shift && !modifiers.ctrl && !modifiers.command)
            });
            if hovered && fresh_wheel || !input.is_scrolling() {
                self.suppress_tail = false;
            }
            let delta = if hovered {
                std::mem::take(&mut input.smooth_scroll_delta.y)
            } else {
                0.0
            };
            (
                if self.suppress_tail {
                    0.0
                } else {
                    -delta * multiplier
                },
                input.stable_dt,
            )
        });
        let offset = self.advance(offset, max_offset, delta, dt);
        if self.remaining != 0.0 {
            ui.ctx().request_repaint();
        }
        offset
    }

    fn advance(&mut self, offset: f32, max_offset: f32, delta: f32, dt: f32) -> f32 {
        let offset = offset.clamp(0.0, max_offset);
        if delta * self.remaining < 0.0 {
            self.remaining = 0.0;
        }
        self.remaining = (self.remaining + delta).clamp(-offset, max_offset - offset);
        let step = if self.remaining.abs() <= 0.05 {
            self.remaining
        } else {
            // За 200 мс проходит 95% оставшегося расстояния без выхода за цель.
            egui::emath::exponential_smooth_factor(0.95, 0.2, dt) * self.remaining
        };
        self.remaining -= step;
        offset + step
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn motion_continues_between_events_then_stops_at_the_requested_distance() {
        let mut scroll = SmoothScroll::default();
        let first = scroll.advance(1000.0, 2000.0, 120.0, 1.0 / 60.0);
        assert!(first > 1000.0 && first < 1120.0);
        let second = scroll.advance(first, 2000.0, 0.0, 1.0 / 60.0);
        assert!(second > first && second < 1120.0);
        let mut offset = second;
        for _ in 0..120 {
            offset = scroll.advance(offset, 2000.0, 0.0, 1.0 / 60.0);
        }
        assert!((offset - 1120.0).abs() < 0.001);
        assert!(scroll.remaining.abs() < f32::EPSILON);
    }

    #[test]
    fn smoothing_is_frame_rate_independent_and_reverses_without_old_momentum() {
        let advance = |fps: u16| {
            let mut scroll = SmoothScroll {
                remaining: 120.0,
                ..SmoothScroll::default()
            };
            let mut offset = 1000.0;
            for _ in 0..fps / 5 {
                offset = scroll.advance(offset, 2000.0, 0.0, 1.0 / f32::from(fps));
            }
            offset
        };
        assert!((advance(60) - advance(120)).abs() < 0.001);
        let mut scroll = SmoothScroll::default();
        let offset = scroll.advance(1000.0, 2000.0, 120.0, 1.0 / 60.0);
        assert!(scroll.advance(offset, 2000.0, -40.0, 1.0 / 60.0) < offset);
        scroll.cancel();
        assert!((scroll.advance(0.0, 2000.0, -120.0, 1.0 / 60.0)).abs() < f32::EPSILON);
        assert!((scroll.advance(2000.0, 2000.0, 120.0, 1.0 / 60.0) - 2000.0).abs() < f32::EPSILON);
        assert!(scroll.remaining.abs() < f32::EPSILON);
    }
}
