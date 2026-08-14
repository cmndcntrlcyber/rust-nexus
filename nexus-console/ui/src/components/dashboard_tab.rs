//! Dashboard tab — embeds the RTPI frontend via iframe.

use leptos::prelude::*;

#[component]
pub fn DashboardTab() -> impl IntoView {
    let rtpi_url = option_env!("NEXUS_RTPI_URL")
        .unwrap_or("http://localhost:5000")
        .to_string();

    view! {
        <div class="tab-content dashboard-tab">
            <iframe
                src=rtpi_url
                class="embed-frame"
                allow="clipboard-read; clipboard-write"
            />
        </div>
    }
}
