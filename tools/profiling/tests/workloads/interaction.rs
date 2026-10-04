use super::*;

const COLUMN_COUNTS: [usize; 5] = [2, 3, 4, 5, 6];
const SCROLL_FRAMES: u32 = 2 * LEG_FRAMES;
const DRAG_FRAMES: u32 = 60;
const TRANSITION_FRAMES: u32 = 2;
const FRAMES_PER_LAYOUT: u32 = TRANSITION_FRAMES + 2 * SCROLL_FRAMES + DRAG_FRAMES;

#[derive(Clone, Copy)]
struct Layout {
    columns: usize,
    viewport: [u16; 2],
}

fn layout(harness: &AppHarness) -> (usize, Rect) {
    {
        let calendar = state(harness).calendar.unwrap();
        (calendar.columns, calendar.rect)
    }
}

fn discover(harness: &mut AppHarness) -> [Layout; 5] {
    // Ширину подбираем по реально использованному числу колонок, включая боковую панель и отступы.
    let mut widths = [None; 5];
    for width in (760_u16..=2600).step_by(32) {
        resize::resize(harness, [width, 800]);
        harness.step();
        let (columns, _) = layout(harness);
        let slot = &mut widths[columns - 2];
        slot.get_or_insert(width + 64);
        if widths.iter().all(Option::is_some) {
            break;
        }
    }
    std::array::from_fn(|index| {
        let viewport = [
            widths[index].expect("Unable to discover calendar columns"),
            800,
        ];
        resize::resize(harness, viewport);
        harness.step();
        assert_eq!(layout(harness).0, COLUMN_COUNTS[index]);
        Layout {
            columns: COLUMN_COUNTS[index],
            viewport,
        }
    })
}

fn check(harness: &AppHarness, target: Layout, selection: DateRange) {
    let snapshot = state(harness);
    assert_eq!(
        layout(harness).0,
        target.columns,
        "Unexpected calendar columns"
    );
    assert_eq!(snapshot.selection, Some(selection));
    let CalendarState { stride, offset, .. } = snapshot.calendar.unwrap();
    assert!(stride.is_finite() && offset.is_finite());
    assert!(snapshot.calendar.unwrap().cached_years <= 5);
    assert_ne!(harness.output().shapes.len(), 0);
    assert!(
        (1901..2100).contains(&snapshot.document.year.get()),
        "Scroll reached a calendar boundary"
    );
}

fn measure(action: impl FnOnce(), samples: &mut Vec<f64>, stage: &mut Vec<f64>) {
    let start = Instant::now();
    action();
    let elapsed = start.elapsed().as_secs_f64() * 1000.0;
    samples.push(elapsed);
    stage.push(elapsed);
}

fn scrolling(
    harness: &mut AppHarness,
    samples: &mut Vec<f64>,
    stage: &mut Vec<f64>,
    mut first: Option<&mut Vec<f64>>,
) {
    let before = offset(harness);
    let mut turn = before;
    for frame in 0..SCROLL_FRAMES {
        let pointer = layout(harness).1.center();
        let start = Instant::now();
        scroll_at(harness, frame, pointer);
        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        samples.push(elapsed);
        if frame == 0
            && let Some(first) = first.as_mut()
        {
            first.push(elapsed);
        } else {
            stage.push(elapsed);
        }
        if frame + 1 == LEG_FRAMES {
            turn = offset(harness);
        }
    }
    assert!(turn > before + 1000.0, "Scroll did not move forward");
    assert!(
        offset(harness) < turn - 1000.0,
        "Scroll did not move backward"
    );
}

struct Timings {
    transition: Vec<f64>,
    first_transition: Vec<f64>,
    before: Vec<f64>,
    drag: Vec<f64>,
    first_after: Vec<f64>,
    after: Vec<f64>,
}

impl Timings {
    fn new(cycles: u32) -> Self {
        let capacity = |frames: u32| usize::try_from(cycles.checked_mul(frames).unwrap()).unwrap();
        Self {
            transition: Vec::with_capacity(capacity(TRANSITION_FRAMES)),
            first_transition: Vec::with_capacity(capacity(1)),
            before: Vec::with_capacity(capacity(SCROLL_FRAMES - 1)),
            drag: Vec::with_capacity(capacity(DRAG_FRAMES)),
            first_after: Vec::with_capacity(capacity(1)),
            after: Vec::with_capacity(capacity(SCROLL_FRAMES - 1)),
        }
    }
}

fn cycle(
    harness: &mut AppHarness,
    layouts: &[Layout; 5],
    selection: DateRange,
    samples: &mut Vec<f64>,
    timings: &mut [Timings; 5],
) {
    for (&target, timing) in layouts.iter().zip(timings) {
        measure(
            || resize::resize(harness, target.viewport),
            samples,
            &mut timing.transition,
        );
        measure(|| harness.step(), samples, &mut timing.transition);
        check(harness, target, selection);
        scrolling(
            harness,
            samples,
            &mut timing.before,
            Some(&mut timing.first_transition),
        );
        for frame in 0..DRAG_FRAMES {
            let width = if frame % 2 == 0 {
                target.viewport[0] - 16
            } else {
                target.viewport[0] + 16
            };
            let height = if frame % 4 < 2 { 776 } else { 824 };
            let viewport = [width, height];
            measure(
                || resize::resize(harness, viewport),
                samples,
                &mut timing.drag,
            );
            assert_eq!(
                harness.ctx.content_rect().size(),
                Vec2::new(f32::from(width), f32::from(height))
            );
            check(harness, target, selection);
        }
        scrolling(
            harness,
            samples,
            &mut timing.after,
            Some(&mut timing.first_after),
        );
        check(harness, target, selection);
    }
}

#[test]
fn interaction_scrolls_and_resizes_through_all_column_counts_without_resetting_state() {
    let mut harness = harness(100);
    let layouts = discover(&mut harness);
    resize::prepare(&mut harness);
    let selection = state(&harness).selection.unwrap();
    let mut samples = Vec::new();
    let mut timings = std::array::from_fn(|_| Timings::new(2));
    for _ in 0..2 {
        cycle(
            &mut harness,
            &layouts,
            selection,
            &mut samples,
            &mut timings,
        );
    }
    assert_eq!(
        samples.len(),
        usize::try_from(2 * 5 * FRAMES_PER_LAYOUT).unwrap()
    );
    assert!(timings.iter().all(|timing| timing.first_after.len() == 2));
}

#[test]
#[ignore = "Run with cargo xtask interaction; optimized CPU workload"]
fn profile_interaction() {
    let cycles = setting("planner_PROFILE_CYCLES", 5).get();
    let events = setting("planner_PROFILE_EVENTS", 1000).get();
    let frames_per_cycle = u32::try_from(COLUMN_COUNTS.len()).unwrap() * FRAMES_PER_LAYOUT;
    let frames = cycles
        .checked_mul(frames_per_cycle)
        .expect("Frame count overflow");
    let mut harness = harness(events);
    let layouts = discover(&mut harness);
    resize::prepare(&mut harness);
    let selection = state(&harness).selection.unwrap();
    let mut samples = Vec::with_capacity(usize::try_from(frames).unwrap());
    let mut timings = std::array::from_fn(|_| Timings::new(cycles));
    for _ in 0..cycles {
        cycle(
            &mut harness,
            &layouts,
            selection,
            &mut samples,
            &mut timings,
        );
    }
    assert_eq!(samples.len(), usize::try_from(frames).unwrap());
    let mut stages = serde_json::Map::new();
    for (&target, timing) in layouts.iter().zip(&mut timings) {
        for (name, samples) in [
            ("transition", &mut timing.transition),
            (
                "first_scroll_after_transition",
                &mut timing.first_transition,
            ),
            ("scroll_before", &mut timing.before),
            ("resize_drag", &mut timing.drag),
            ("first_scroll_after_resize", &mut timing.first_after),
            ("scroll_after", &mut timing.after),
        ] {
            let mut stats = summary(samples);
            stats["samples"] = serde_json::json!(samples.len());
            stages.insert(format!("{}-columns/{name}", target.columns), stats);
        }
    }
    let mut result = summary(&mut samples);
    result.as_object_mut().unwrap().extend(
        serde_json::json!({
            "schema": 1,
            "scenario": "continuous-interaction-v1",
            "events": events,
            "cycles": cycles,
            "frames": frames,
            "viewport": [1440, 960],
            "viewports": layouts.map(|target| target.viewport),
            "anchor_date": selection.start(),
            "step_dt": STEP_DT,
            "warmup_frames": 8,
            "route": {
                "columns": COLUMN_COUNTS,
                "transition_frames": TRANSITION_FRAMES,
                "scroll_frames": SCROLL_FRAMES,
                "resize_frames": DRAG_FRAMES,
                "width_delta": 16,
                "heights": [776, 824],
                "shared_planner": true,
                "warmup_route_cycles": 0,
            },
            "stages": stages,
        })
        .as_object()
        .unwrap()
        .clone(),
    );
    save_result(&result);
}
