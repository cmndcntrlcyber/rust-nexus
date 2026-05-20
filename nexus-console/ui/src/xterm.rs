//! Thin wasm-bindgen bindings to xterm.js (vendored via Trunk pre-build).

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    /// `window.Terminal` from xterm.js.
    #[wasm_bindgen(js_namespace = window, js_name = Terminal)]
    pub type XtermTerminal;

    #[wasm_bindgen(constructor, js_namespace = window, js_name = Terminal)]
    pub fn new(opts: &JsValue) -> XtermTerminal;

    #[wasm_bindgen(method, js_name = open)]
    pub fn open(this: &XtermTerminal, element: &web_sys::Element);

    #[wasm_bindgen(method, js_name = write)]
    pub fn write_str(this: &XtermTerminal, data: &str);

    #[wasm_bindgen(method, js_name = write)]
    pub fn write_bytes(this: &XtermTerminal, data: &js_sys::Uint8Array);

    #[wasm_bindgen(method, js_name = onData)]
    pub fn on_data(this: &XtermTerminal, handler: &Closure<dyn FnMut(JsValue)>) -> JsValue;

    #[wasm_bindgen(method, js_name = onResize)]
    pub fn on_resize(this: &XtermTerminal, handler: &Closure<dyn FnMut(JsValue)>) -> JsValue;

    #[wasm_bindgen(method, js_name = loadAddon)]
    pub fn load_addon(this: &XtermTerminal, addon: &JsValue);

    #[wasm_bindgen(method, getter)]
    pub fn cols(this: &XtermTerminal) -> u16;

    #[wasm_bindgen(method, getter)]
    pub fn rows(this: &XtermTerminal) -> u16;

    #[wasm_bindgen(js_namespace = window, js_name = FitAddon)]
    pub type FitAddon;

    #[wasm_bindgen(constructor, js_namespace = window, js_name = FitAddon)]
    pub fn fit_addon_new() -> FitAddon;

    #[wasm_bindgen(method, js_name = fit)]
    pub fn fit(this: &FitAddon);
}

/// Construct a Terminal with sensible defaults.
#[must_use]
pub fn build_terminal() -> XtermTerminal {
    let opts = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&opts, &JsValue::from_str("convertEol"), &JsValue::from_bool(true));
    let _ = js_sys::Reflect::set(&opts, &JsValue::from_str("cursorBlink"), &JsValue::from_bool(true));
    let _ = js_sys::Reflect::set(
        &opts,
        &JsValue::from_str("fontFamily"),
        &JsValue::from_str("Menlo, Consolas, monospace"),
    );
    let _ = js_sys::Reflect::set(&opts, &JsValue::from_str("fontSize"), &JsValue::from_f64(13.0));
    let theme = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&theme, &JsValue::from_str("background"), &JsValue::from_str("#000000"));
    let _ = js_sys::Reflect::set(&theme, &JsValue::from_str("foreground"), &JsValue::from_str("#e6edf3"));
    let _ = js_sys::Reflect::set(&opts, &JsValue::from_str("theme"), &theme);
    XtermTerminal::new(&opts)
}
