#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use planner::Planner;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    eframe::run_native(
        "planner",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1440.0, 960.0])
                .with_min_inner_size([760.0, 600.0]),
            ..Default::default()
        },
        Box::new(|cc| Ok(Box::new(Planner::new(cc)))),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {
    use wasm_bindgen::JsCast as _;
    wasm_bindgen_futures::spawn_local(async {
        let result = async {
            let canvas = web_sys::window()
                .and_then(|window| window.document())
                .and_then(|document| document.get_element_by_id("planner"))
                .ok_or_else(|| wasm_bindgen::JsValue::from_str("Calendar canvas not found"))?
                .dyn_into::<web_sys::HtmlCanvasElement>()?;
            eframe::WebRunner::new()
                .start(
                    canvas,
                    eframe::WebOptions::default(),
                    Box::new(|cc| Ok(Box::new(Planner::new(cc)))),
                )
                .await
        }
        .await;
        if let Err(error) = result {
            web_sys::console::error_1(&error);
            if let Some(element) = web_sys::window()
                .and_then(|window| window.document())
                .and_then(|document| document.get_element_by_id("loading"))
            {
                element.set_text_content(Some(
                    "Unable to start planner. Check WebGL support and reload.",
                ));
            }
        } else if let Some(document) = web_sys::window().and_then(|window| window.document()) {
            for id in ["loading", "about"] {
                if let Some(element) = document.get_element_by_id(id) {
                    element.remove();
                }
            }
        }
    });
}
