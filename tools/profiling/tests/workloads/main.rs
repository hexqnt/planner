use std::{num::NonZeroU32, time::Instant};

use chrono::NaiveDate;

use egui::{Rect, Vec2};
use planner::{
    Planner,
    testing::{self, CalendarState, DateRange},
};
use planner_test_support::{AppHarness, STEP_DT, state};

mod interaction;
mod resize;
mod startup;

const LEG_FRAMES: u32 = 120;

fn harness(count: u32) -> AppHarness {
    planner_test_support::harness(
        planner_test_support::profile_document(count),
        Vec2::new(1440.0, 960.0),
    )
}

fn offset(harness: &AppHarness) -> f32 {
    state(harness).calendar.unwrap().offset
}

fn scroll(harness: &mut AppHarness, frame: u32) {
    scroll_at(harness, frame, egui::pos2(1050.0, 480.0));
}

fn scroll_at(harness: &mut AppHarness, frame: u32, pointer: egui::Pos2) {
    // Каждое событие попадает ровно в один кадр; виртуальное время не зависит от скорости CPU.
    let direction = if frame % (2 * LEG_FRAMES) < LEG_FRAMES {
        -1.0
    } else {
        1.0
    };
    harness.input_mut().events.extend([
        egui::Event::PointerMoved(pointer),
        egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            phase: egui::TouchPhase::Move,
            delta: Vec2::new(0.0, direction * 24.0),
            modifiers: egui::Modifiers::NONE,
        },
    ]);
    harness.step();
}

#[test]
fn scrolling_scenario_moves_in_both_directions_and_crosses_years() {
    let mut harness = harness(100);
    harness.run_steps(8);
    let start = offset(&harness);
    let year = state(&harness).document.year;
    for frame in 0..LEG_FRAMES {
        scroll(&mut harness, frame);
    }
    let end = offset(&harness);
    assert!(end > start + 1000.0);
    assert!(state(&harness).document.year.get() > year.get());
    for frame in LEG_FRAMES..2 * LEG_FRAMES {
        scroll(&mut harness, frame);
    }
    assert!(offset(&harness) < end - 1000.0);
    assert!(state(&harness).calendar.unwrap().cached_years <= 5);
}

fn setting(name: &str, default: u32) -> NonZeroU32 {
    std::env::var(name).map_or_else(
        |_| NonZeroU32::new(default).unwrap(),
        |value| {
            value
                .parse()
                .unwrap_or_else(|_| panic!("{name} must be a positive integer"))
        },
    )
}

#[test]
#[ignore = "Run with cargo xtask scroll; optimized CPU workload"]
fn profile_scroll() {
    let cycles = setting("planner_PROFILE_CYCLES", 20).get();
    let events = setting("planner_PROFILE_EVENTS", 1000).get();
    let frames = cycles
        .checked_mul(2 * LEG_FRAMES)
        .expect("Frame count overflow");
    let mut harness = harness(events);
    harness.run_steps(8);
    for frame in 0..2 * LEG_FRAMES {
        scroll(&mut harness, frame);
    }
    let mut samples = Vec::with_capacity(usize::try_from(frames).unwrap());
    let mut min_offset = f32::MAX;
    let mut max_offset = f32::MIN;
    for frame in 0..frames {
        let start = Instant::now();
        scroll(&mut harness, frame);
        samples.push(start.elapsed().as_secs_f64() * 1000.0);
        let offset = offset(&harness);
        min_offset = min_offset.min(offset);
        max_offset = max_offset.max(offset);
    }
    assert!(
        max_offset - min_offset > 1000.0,
        "Scroll scenario did not move"
    );
    let mut result = serde_json::json!({
        "schema": 1,
        "scenario": "continuous-scroll-v1",
        "events": events,
        "cycles": cycles,
        "frames": frames,
        "viewport": [1440, 960],
        "step_dt": STEP_DT,
        "warmup_frames": 8 + 2 * LEG_FRAMES,
        "offset_span": max_offset - min_offset,
    });
    result
        .as_object_mut()
        .unwrap()
        .extend(summary(&mut samples).as_object().unwrap().clone());
    save_result(&result);
}

fn summary(samples: &mut [f64]) -> serde_json::Value {
    let total: f64 = samples.iter().sum();
    samples.sort_unstable_by(f64::total_cmp);
    let percentile = |percent: usize| samples[(samples.len() - 1) * percent / 100];
    serde_json::json!({
        "mean_ms": total / f64::from(u32::try_from(samples.len()).unwrap()),
        "p50_ms": percentile(50),
        "p95_ms": percentile(95),
        "p99_ms": percentile(99),
        "max_ms": samples.last().unwrap(),
        "total_ms": total,
    })
}

fn save_result(result: &serde_json::Value) {
    let json = serde_json::to_string_pretty(&result).unwrap();
    if let Some(path) = std::env::var_os("planner_PROFILE_OUTPUT") {
        std::fs::write(path, &json).expect("Unable to write profile metrics");
    }
    println!("{json}");
}
