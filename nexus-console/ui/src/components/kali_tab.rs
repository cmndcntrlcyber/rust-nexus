//! Kali tab — embeds the KasmVNC desktop via iframe.

use leptos::prelude::*;

#[component]
pub fn KaliTab() -> impl IntoView {
    let kasm_url = option_env!("NEXUS_KASM_URL")
        .unwrap_or("https://localhost:6901")
        .to_string();

    let (loaded, set_loaded) = signal(false);

    view! {
        <div class="tab-content kali-tab">
            {move || {
                if !loaded.get() {
                    Some(view! {
                        <div class="loading-overlay">
                            <p>"Connecting to Kali desktop..."</p>
                            <p class="hint">"Accept the self-signed certificate if prompted."</p>
                        </div>
                    })
                } else {
                    None
                }
            }}
            <iframe
                src=kasm_url
                class="embed-frame"
                allow="clipboard-read; clipboard-write; autoplay"
                on:load=move |_| set_loaded.set(true)
            />
        </div>
    }
}
