use std::{cell::RefCell, rc::Rc};

use wasm_bindgen::{JsCast as _, JsValue, closure::Closure};

type FileResult = std::result::Result<String, String>;

#[derive(Default)]
pub(super) struct BrowserFiles {
    state: Rc<RefCell<ReadState>>,
    chooser: Option<Chooser>,
}

#[derive(Default)]
enum ReadState {
    #[default]
    Idle,
    Reading,
    Ready(FileResult),
}

struct Chooser {
    input: web_sys::HtmlInputElement,
    _handler: Closure<dyn FnMut(web_sys::Event)>,
}

impl Drop for Chooser {
    fn drop(&mut self) {
        self.input.set_onchange(None);
        self.input.set_oncancel(None);
        self.input.remove();
    }
}

fn js_error(error: impl Into<JsValue>) -> String {
    let error = error.into();
    format!("Browser file operation failed: {error:?}")
}

fn document() -> std::result::Result<web_sys::Document, String> {
    web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| "Browser document unavailable".into())
}

impl BrowserFiles {
    pub(super) fn reset(&mut self) {
        self.chooser = None;
        // Незавершённое чтение сохраняет старый Rc и не может завершить следующую операцию.
        self.state = Rc::default();
    }

    pub(super) fn is_reading(&self) -> bool {
        matches!(*self.state.borrow(), ReadState::Reading)
    }

    pub(super) fn take_result(&mut self) -> Option<FileResult> {
        if self.is_reading() {
            return None;
        }
        self.chooser = None;
        match self.state.replace(ReadState::Idle) {
            ReadState::Ready(result) => Some(result),
            ReadState::Idle | ReadState::Reading => None,
        }
    }

    pub(super) fn pick(&mut self, ctx: egui::Context) -> std::result::Result<(), String> {
        self.reset();
        let document = document()?;
        let input = document
            .create_element("input")
            .map_err(js_error)?
            .dyn_into::<web_sys::HtmlInputElement>()
            .map_err(js_error)?;
        input.set_type("file");
        input.set_accept(".ics,text/calendar");
        input.set_attribute("hidden", "").map_err(js_error)?;
        document
            .body()
            .ok_or("Browser body unavailable")?
            .append_child(&input)
            .map_err(js_error)?;
        let state = self.state.clone();
        let handler = Closure::new(move |event: web_sys::Event| {
            let file = event
                .target()
                .and_then(|target| target.dyn_into::<web_sys::HtmlInputElement>().ok())
                .and_then(|input| input.files())
                .and_then(|files| files.get(0));
            let Some(file) = file else {
                *state.borrow_mut() = ReadState::Idle;
                ctx.request_repaint();
                return;
            };
            let state = state.clone();
            let ctx = ctx.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let text = wasm_bindgen_futures::JsFuture::from(file.text())
                    .await
                    .map_err(js_error)
                    .and_then(|text| {
                        text.as_string()
                            .ok_or_else(|| "Unable to read file text".into())
                    });
                *state.borrow_mut() = ReadState::Ready(text);
                ctx.request_repaint();
            });
        });
        input.set_onchange(Some(handler.as_ref().unchecked_ref()));
        input.set_oncancel(Some(handler.as_ref().unchecked_ref()));
        *self.state.borrow_mut() = ReadState::Reading;
        input.click();
        self.chooser = Some(Chooser {
            input,
            _handler: handler,
        });
        Ok(())
    }
}

pub(super) fn download(text: &str) -> std::result::Result<(), String> {
    let parts = js_sys::Array::new();
    parts.push(&JsValue::from_str(text));
    let options = web_sys::BlobPropertyBag::new();
    options.set_type("text/calendar;charset=utf-8");
    let blob =
        web_sys::Blob::new_with_str_sequence_and_options(&parts, &options).map_err(js_error)?;
    let url = web_sys::Url::create_object_url_with_blob(&blob).map_err(js_error)?;
    let result = (|| {
        let document = document()?;
        let link = document
            .create_element("a")
            .map_err(js_error)?
            .dyn_into::<web_sys::HtmlAnchorElement>()
            .map_err(js_error)?;
        link.set_href(&url);
        link.set_download("planner.ics");
        document
            .body()
            .ok_or("Browser body unavailable")?
            .append_child(&link)
            .map_err(js_error)?;
        link.click();
        link.remove();
        Ok(())
    })();
    web_sys::Url::revoke_object_url(&url).map_err(js_error)?;
    result
}
