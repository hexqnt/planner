use super::*;

const VIEWPORTS: [[u16; 2]; 6] = [
    [760, 600],
    [1120, 800],
    [1920, 1080],
    [1600, 700],
    [960, 1000],
    [1440, 960],
];
const FRAMES_PER_RESIZE: u32 = 2;
const ANCHOR_DATE: NaiveDate = NaiveDate::from_ymd_opt(2026, 6, 15).expect("Valid anchor date");

pub fn prepare(harness: &mut AppHarness) -> f32 {
    harness
        .state_mut()
        .as_mut()
        .unwrap()
        .select_date(ANCHOR_DATE)
        .unwrap();
    harness.run_steps(8);
    let CalendarState { stride, offset, .. } = state(harness).calendar.unwrap();
    offset / stride
}

pub fn resize(harness: &mut AppHarness, [width, height]: [u16; 2]) {
    harness.set_size(Vec2::new(f32::from(width), f32::from(height)));
    harness.step();
}

fn check(harness: &AppHarness, viewport: [u16; 2], anchor: f32) -> f32 {
    let snapshot = state(harness);
    let size = Vec2::new(f32::from(viewport[0]), f32::from(viewport[1]));
    assert_eq!(
        harness.ctx.content_rect().size(),
        size,
        "Viewport did not resize"
    );
    let CalendarState { stride, offset, .. } = snapshot.calendar.unwrap();
    assert!(
        (offset / stride - anchor).abs() < 0.002,
        "Resize changed the calendar anchor"
    );
    let date = ANCHOR_DATE;
    assert_eq!(snapshot.selection, Some(DateRange::between(date, date)));
    assert_eq!(snapshot.document.year.get(), 2026);
    assert!(snapshot.calendar.unwrap().cached_years <= 5);
    assert_ne!(harness.output().shapes.len(), 0);
    stride
}

#[test]
fn resize_changes_calendar_layout_and_preserves_date_and_anchor() {
    let mut harness = harness(100);
    let anchor = prepare(&mut harness);
    let initial_stride = state(&harness).calendar.unwrap().stride;
    let mut changed = false;
    for viewport in VIEWPORTS {
        resize(&mut harness, viewport);
        harness.step();
        let stride = check(&harness, viewport, anchor);
        changed |= (stride - initial_stride).abs() > 1.0;
    }
    assert!(changed, "Calendar layout did not change during resize");
    let final_stride = state(&harness).calendar.unwrap().stride;
    assert!(
        (final_stride - initial_stride).abs() < 0.01,
        "Original layout did not return"
    );
}

#[test]
#[ignore = "Run with cargo xtask resize; optimized CPU workload"]
fn profile_resize() {
    let cycles = setting("planner_PROFILE_CYCLES", 100).get();
    let events = setting("planner_PROFILE_EVENTS", 1000).get();
    let frames_per_cycle = u32::try_from(VIEWPORTS.len()).unwrap() * FRAMES_PER_RESIZE;
    let frames = cycles
        .checked_mul(frames_per_cycle)
        .expect("Frame count overflow");
    let mut harness = harness(events);
    let anchor = prepare(&mut harness);
    // Прогреваем все варианты компоновки до измерений.
    for viewport in VIEWPORTS {
        resize(&mut harness, viewport);
        harness.step();
        check(&harness, viewport, anchor);
    }
    let mut samples = Vec::with_capacity(usize::try_from(frames).unwrap());
    let mut per_viewport: [Vec<f64>; VIEWPORTS.len()] = std::array::from_fn(|_| {
        Vec::with_capacity(usize::try_from(cycles * FRAMES_PER_RESIZE).unwrap())
    });
    let mut min_stride = f32::MAX;
    let mut max_stride = f32::MIN;
    for _ in 0..cycles {
        for (viewport, timings) in VIEWPORTS.into_iter().zip(&mut per_viewport) {
            let start = Instant::now();
            resize(&mut harness, viewport);
            let elapsed = start.elapsed().as_secs_f64() * 1000.0;
            samples.push(elapsed);
            timings.push(elapsed);
            let start = Instant::now();
            harness.step();
            let elapsed = start.elapsed().as_secs_f64() * 1000.0;
            samples.push(elapsed);
            timings.push(elapsed);
            let stride = check(&harness, viewport, anchor);
            min_stride = min_stride.min(stride);
            max_stride = max_stride.max(stride);
        }
    }
    assert!(
        max_stride - min_stride > 1.0,
        "Calendar layout did not change during resize"
    );
    let stages: serde_json::Map<_, _> = VIEWPORTS
        .into_iter()
        .zip(&mut per_viewport)
        .map(|([width, height], timings)| (format!("{width}x{height}"), summary(timings)))
        .collect();
    let mut result = summary(&mut samples);
    result.as_object_mut().unwrap().extend(
        serde_json::json!({
            "schema": 1,
            "scenario": "continuous-resize-v1",
            "events": events,
            "cycles": cycles,
            "frames": frames,
            "viewport": [1440, 960],
            "viewports": VIEWPORTS,
            "frames_per_resize": FRAMES_PER_RESIZE,
            "anchor_date": ANCHOR_DATE,
            "step_dt": STEP_DT,
            "warmup_frames": 8 + frames_per_cycle,
            "stride_span": max_stride - min_stride,
            "stages": stages,
        })
        .as_object()
        .unwrap()
        .clone(),
    );
    save_result(&result);
}
