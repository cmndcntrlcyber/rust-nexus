//! Generic iframe embed component reused by all tunnel-backed tabs.
//!
//! Displays a loading overlay until the iframe fires its `load` event,
//! then shows the embedded service. Follows the same pattern as
//! `DashboardTab` and `KaliTab` but accepts URL and label as props.

use leptos::prelude::*;

#[component]
pub fn ServiceTab(url: String, label: String) -> impl IntoView {
    let (loaded, set_loaded) = signal(false);
    let loading_label = label.clone();
    view! {
        <div class="tab-content service-tab">
            {move || {
                if !loaded.get() {
                    Some(view! {
                        <div class="loading-overlay">
                            <p>{format!("Loading {}...", loading_label)}</p>
                        </div>
                    })
                } else {
                    None
                }
            }}
            <iframe
                src=url
                class="embed-frame"
                allow="clipboard-read; clipboard-write; autoplay"
                on:load=move |_| set_loaded.set(true)
            />
        </div>
    }
}
