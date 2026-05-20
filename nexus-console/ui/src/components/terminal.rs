//! Terminal pane.

use std::cell::RefCell;
use std::rc::Rc;

use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Serialize;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use crate::tauri_api;
use crate::types::{ShellErrorPayload, ShellExitPayload, ShellOutputPayload};
use crate::xterm::{build_terminal, FitAddon, XtermTerminal};

#[derive(Serialize)]
struct OpenArgs<'a> {
    #[serde(rename = "targetPeerId")]
    target_peer_id: Option<&'a str>,
    cols: u16,
    rows: u16,
}

#[derive(Serialize)]
struct SendBytesArgs {
    #[serde(rename = "sessionId")]
    session_id: u64,
    bytes: Vec<u8>,
}

#[derive(Serialize)]
struct ResizeArgs {
    #[serde(rename = "sessionId")]
    session_id: u64,
    cols: u16,
    rows: u16,
}

#[derive(Serialize)]
struct CloseArgs {
    #[serde(rename = "sessionId")]
    session_id: u64,
}

#[component]
pub fn Terminal(
    /// Hex peer id of the selected agent.
    selected: ReadSignal<Option<String>>,
    /// Setter for the live session id.
    set_session_id_signal: WriteSignal<Option<u64>>,
) -> impl IntoView {
    let host_ref = NodeRef::<leptos::html::Div>::new();
    let state: Rc<RefCell<TerminalState>> = Rc::new(RefCell::new(TerminalState::default()));

    let state_for_effect = Rc::clone(&state);
    Effect::new(move |_| {
        let Some(host) = host_ref.get() else { return };
        let Some(target) = selected.get() else { return };
        let host_element: web_sys::Element = host.unchecked_into();

        let st = Rc::clone(&state_for_effect);
        {
            let mut guard = st.borrow_mut();
            guard.teardown();
        }

        let term: Rc<XtermTerminal> = Rc::new(build_terminal());
        let fit_addon: Rc<FitAddon> = Rc::new(FitAddon::fit_addon_new());
        term.load_addon(fit_addon.as_ref().as_ref());
        term.open(&host_element);
        fit_addon.fit();
        let cols = term.cols();
        let rows = term.rows();

        let target_owned = target.clone();
        let st_for_open = Rc::clone(&st);
        spawn_local(async move {
            let args = OpenArgs {
                target_peer_id: Some(target_owned.as_str()),
                cols,
                rows,
            };
            match tauri_api::invoke::<_, u64>("open_shell_session", &args).await {
                Ok(session_id) => {
                    set_session_id_signal.set(Some(session_id));
                    st_for_open.borrow_mut().session_id = Some(session_id);
                }
                Err(err) => {
                    web_sys::console::warn_1(&JsValue::from_str(&format!(
                        "open_shell_session failed: {err}"
                    )));
                }
            }
        });

        let st_for_data = Rc::clone(&st);
        let on_data = Closure::wrap(Box::new(move |arg: JsValue| {
            let s = arg.as_string().unwrap_or_default();
            let bytes = s.into_bytes();
            let Some(sid) = st_for_data.borrow().session_id else {
                return;
            };
            spawn_local(async move {
                let args = SendBytesArgs {
                    session_id: sid,
                    bytes,
                };
                let _ = tauri_api::invoke::<_, ()>("send_shell_bytes", &args).await;
            });
        }) as Box<dyn FnMut(JsValue)>);
        term.on_data(&on_data);

        let st_for_resize = Rc::clone(&st);
        let on_resize = Closure::wrap(Box::new(move |arg: JsValue| {
            let Some(sid) = st_for_resize.borrow().session_id else {
                return;
            };
            let cols = js_sys::Reflect::get(&arg, &JsValue::from_str("cols"))
                .ok()
                .and_then(|v| v.as_f64())
                .map_or(80u16, |f| f as u16);
            let rows = js_sys::Reflect::get(&arg, &JsValue::from_str("rows"))
                .ok()
                .and_then(|v| v.as_f64())
                .map_or(24u16, |f| f as u16);
            spawn_local(async move {
                let args = ResizeArgs {
                    session_id: sid,
                    cols,
                    rows,
                };
                let _ = tauri_api::invoke::<_, ()>("resize_shell", &args).await;
            });
        }) as Box<dyn FnMut(JsValue)>);
        term.on_resize(&on_resize);

        let term_for_output = Rc::clone(&term);
        let st_for_output = Rc::clone(&st);
        let st_for_output_stash = Rc::clone(&st);
        spawn_local(async move {
            match tauri_api::listen::<ShellOutputPayload, _>("shell-output", move |payload| {
                let active = st_for_output.borrow().session_id;
                if Some(payload.session_id) != active {
                    return;
                }
                let arr = js_sys::Uint8Array::from(payload.bytes.as_slice());
                term_for_output.write_bytes(&arr);
            })
            .await
            {
                Ok(c) => st_for_output_stash.borrow_mut().output_listener = Some(c),
                Err(err) => web_sys::console::warn_1(&JsValue::from_str(&format!(
                    "listen shell-output: {err}"
                ))),
            }
        });

        let term_for_err = Rc::clone(&term);
        let st_for_err = Rc::clone(&st);
        let st_for_err_stash = Rc::clone(&st);
        spawn_local(async move {
            match tauri_api::listen::<ShellErrorPayload, _>("shell-error", move |payload| {
                let active = st_for_err.borrow().session_id;
                if Some(payload.session_id) != active {
                    return;
                }
                term_for_err.write_str(&format!("\r\n[shell error: {}]\r\n", payload.message));
            })
            .await
            {
                Ok(c) => st_for_err_stash.borrow_mut().error_listener = Some(c),
                Err(err) => web_sys::console::warn_1(&JsValue::from_str(&format!(
                    "listen shell-error: {err}"
                ))),
            }
        });

        let term_for_exit = Rc::clone(&term);
        let st_for_exit = Rc::clone(&st);
        let st_for_exit_stash = Rc::clone(&st);
        spawn_local(async move {
            match tauri_api::listen::<ShellExitPayload, _>("shell-exit", move |payload| {
                let active = st_for_exit.borrow().session_id;
                if Some(payload.session_id) != active {
                    return;
                }
                let msg = match payload.code {
                    Some(c) => format!("\r\n[session exited with code {c}]\r\n"),
                    None => "\r\n[session terminated]\r\n".to_string(),
                };
                term_for_exit.write_str(&msg);
                set_session_id_signal.set(None);
            })
            .await
            {
                Ok(c) => st_for_exit_stash.borrow_mut().exit_listener = Some(c),
                Err(err) => web_sys::console::warn_1(&JsValue::from_str(&format!(
                    "listen shell-exit: {err}"
                ))),
            }
        });

        let mut guard = state_for_effect.borrow_mut();
        guard.terminal = Some(term);
        guard.fit_addon = Some(fit_addon);
        guard.on_data = Some(on_data);
        guard.on_resize = Some(on_resize);
    });

    view! {
        <section class="terminal-pane">
            <Show
                when=move || selected.get().is_some()
                fallback=move || {
                    view! {
                        <div class="empty-terminal">
                            <p>"Select an agent on the left to open a shell."</p>
                            <p style="font-size: 11px; margin-top: 12px;">
                                "v1.1 note: interactive shells require the agent to register via the new A2A path. Agents using only the overlay's existing RegisterAgent flow appear here but the v1.2 follow-up adds the bidi back-channel to interact with them."
                            </p>
                        </div>
                    }
                }
            >
                <div node_ref=host_ref id="terminal-host"></div>
            </Show>
        </section>
    }
}

#[derive(Default)]
struct TerminalState {
    session_id: Option<u64>,
    terminal: Option<Rc<XtermTerminal>>,
    fit_addon: Option<Rc<FitAddon>>,
    on_data: Option<Closure<dyn FnMut(JsValue)>>,
    on_resize: Option<Closure<dyn FnMut(JsValue)>>,
    output_listener: Option<Closure<dyn FnMut(JsValue)>>,
    exit_listener: Option<Closure<dyn FnMut(JsValue)>>,
    error_listener: Option<Closure<dyn FnMut(JsValue)>>,
}

impl TerminalState {
    fn teardown(&mut self) {
        if let Some(sid) = self.session_id.take() {
            spawn_local(async move {
                let args = CloseArgs { session_id: sid };
                let _ = tauri_api::invoke::<_, ()>("close_shell_session", &args).await;
            });
        }
        self.terminal = None;
        self.fit_addon = None;
        self.on_data = None;
        self.on_resize = None;
        self.output_listener = None;
        self.exit_listener = None;
        self.error_listener = None;
    }
}
