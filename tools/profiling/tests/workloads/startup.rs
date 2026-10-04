use super::*;

struct Storage(String);

impl eframe::Storage for Storage {
    fn get_string(&self, key: &str) -> Option<String> {
        (key == "planner.document.v1").then(|| self.0.clone())
    }

    fn set_string(&mut self, _key: &str, _value: String) {
        panic!("Startup scenario must not save data");
    }

    fn remove_string(&mut self, _key: &str) {
        panic!("Startup scenario must not remove data");
    }

    fn flush(&mut self) {
        panic!("Startup scenario must not flush data");
    }
}

struct StartupSample {
    planner: Planner,
    output: egui::FullOutput,
    load_and_init_ms: f64,
    first_ui_ms: f64,
    startup_ms: f64,
}

fn milliseconds(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}

fn startup(storage: &Storage) -> StartupSample {
    // Инициализация, как в eframe, предшествует первому UI-проходу: шрифты доступны сразу.
    let start = Instant::now();
    let ctx = egui::Context::default();
    let init_start = Instant::now();
    let mut planner = testing::from_storage(Some(storage), &ctx);
    let load_and_init_ms = milliseconds(init_start);
    let input = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(
            egui::Pos2::ZERO,
            Vec2::new(1440.0, 960.0),
        )),
        time: Some(0.0),
        predicted_dt: STEP_DT,
        ..Default::default()
    };
    let ui_start = Instant::now();
    let output = ctx.run_ui(input, |ui| testing::render(&mut planner, ui));
    let first_ui_ms = milliseconds(ui_start);
    let startup_ms = milliseconds(start);
    StartupSample {
        planner,
        output,
        load_and_init_ms,
        first_ui_ms,
        startup_ms,
    }
}

#[test]
fn startup_loads_saved_events_and_builds_the_initial_calendar() {
    let storage =
        Storage(serde_json::to_string(&planner_test_support::profile_document(100)).unwrap());
    let result = startup(&storage);
    let snapshot = result.planner.inspect();
    assert_eq!(snapshot.document.events.len(), 100);
    assert_eq!(snapshot.document.year.get(), 2026);
    assert!(snapshot.notice.is_none());
    assert!(snapshot.calendar.is_some());
    assert!(snapshot.calendar.unwrap().has_events);
    assert_ne!(result.output.shapes.len(), 0);
    assert!(result.startup_ms >= result.load_and_init_ms + result.first_ui_ms);
    result.output.drop_without_applying_deltas();
}

#[test]
#[ignore = "Run with cargo xtask startup; optimized CPU workload"]
fn profile_startup() {
    let cycles = setting("planner_PROFILE_CYCLES", 100).get();
    let events = setting("planner_PROFILE_EVENTS", 1000).get();
    // Генерация и сериализация эталонного документа не входят в JSON-метрики запуска.
    let storage =
        Storage(serde_json::to_string(&planner_test_support::profile_document(events)).unwrap());
    let capacity = usize::try_from(cycles).unwrap();
    let mut samples = Vec::with_capacity(capacity);
    let mut initialization = Vec::with_capacity(capacity);
    let mut first_ui = Vec::with_capacity(capacity);
    for _ in 0..cycles {
        let result = startup(&storage);
        let snapshot = result.planner.inspect();
        assert_eq!(
            snapshot.document.events.len(),
            usize::try_from(events).unwrap()
        );
        assert!(snapshot.notice.is_none(), "Saved document did not load");
        assert!(snapshot.calendar.is_some(), "Calendar was not initialized");
        assert!(!result.output.shapes.is_empty(), "Initial UI was not built");
        samples.push(result.startup_ms);
        initialization.push(result.load_and_init_ms);
        first_ui.push(result.first_ui_ms);
        result.output.drop_without_applying_deltas();
    }
    let mut result = summary(&mut samples);
    result.as_object_mut().unwrap().extend(
        serde_json::json!({
            "schema": 1,
            "scenario": "startup-ui-v1",
            "events": events,
            "cycles": cycles,
            "frames": cycles,
            "viewport": [1440, 960],
            "step_dt": STEP_DT,
            "warmup_frames": 0,
            "storage": "saved-json-in-memory",
            "stages": {
                "load_and_init": summary(&mut initialization),
                "first_ui": summary(&mut first_ui),
            },
        })
        .as_object()
        .unwrap()
        .clone(),
    );
    save_result(&result);
}
