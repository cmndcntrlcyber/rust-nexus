//! Thin bindings to Tauri v2 `window.__TAURI__` JS API.

use serde::de::DeserializeOwned;
use serde::Serialize;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke, catch)]
    async fn tauri_invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"], js_name = listen, catch)]
    async fn tauri_listen(event: &str, handler: &Closure<dyn FnMut(JsValue)>) -> Result<JsValue, JsValue>;
}

/// Call a Tauri command.
pub async fn invoke<A: Serialize, R: DeserializeOwned>(cmd: &str, args: &A) -> Result<R, String> {
    let args_value =
        serde_wasm_bindgen::to_value(args).map_err(|e| format!("encode args: {e}"))?;
    let result = tauri_invoke(cmd, args_value)
        .await
        .map_err(|e| format!("invoke {cmd}: {}", js_value_to_string(&e)))?;
    serde_wasm_bindgen::from_value(result).map_err(|e| format!("decode {cmd} result: {e}"))
}

/// Invoke with no args.
pub async fn invoke_no_args<R: DeserializeOwned>(cmd: &str) -> Result<R, String> {
    let args = JsValue::from(js_sys::Object::new());
    let result = tauri_invoke(cmd, args)
        .await
        .map_err(|e| format!("invoke {cmd}: {}", js_value_to_string(&e)))?;
    serde_wasm_bindgen::from_value(result).map_err(|e| format!("decode {cmd} result: {e}"))
}

/// Listen for a Tauri event.
pub async fn listen<P, F>(event: &str, mut handler: F) -> Result<Closure<dyn FnMut(JsValue)>, String>
where
    P: DeserializeOwned + 'static,
    F: FnMut(P) + 'static,
{
    let event_name = event.to_string();
    let closure = Closure::wrap(Box::new(move |js_event: JsValue| {
        let payload = match js_sys::Reflect::get(&js_event, &JsValue::from_str("payload")) {
            Ok(p) => p,
            Err(err) => {
                web_sys::console::warn_1(&JsValue::from_str(&format!(
                    "tauri_api: missing payload on {event_name}: {err:?}"
                )));
                return;
            }
        };
        match serde_wasm_bindgen::from_value::<P>(payload) {
            Ok(p) => handler(p),
            Err(err) => {
                web_sys::console::warn_1(&JsValue::from_str(&format!(
                    "tauri_api: decode payload for {event_name}: {err}"
                )));
            }
        }
    }) as Box<dyn FnMut(JsValue)>);

    tauri_listen(event, &closure)
        .await
        .map_err(|e| format!("listen {event}: {}", js_value_to_string(&e)))?;
    Ok(closure)
}

fn js_value_to_string(v: &JsValue) -> String {
    if let Some(s) = v.as_string() {
        s
    } else if let Some(obj) = v.dyn_ref::<js_sys::Object>() {
        js_sys::JSON::stringify(obj)
            .ok()
            .and_then(|s| s.as_string())
            .unwrap_or_else(|| format!("{v:?}"))
    } else {
        format!("{v:?}")
    }
}
