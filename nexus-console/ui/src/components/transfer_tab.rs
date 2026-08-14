//! Transfer tab — signed file transfer + network relay (stub).

use leptos::prelude::*;

#[component]
pub fn TransferTab() -> impl IntoView {
    view! {
        <div class="tab-content transfer-tab">
            <div class="transfer-header">
                <h3>"Signed File Transfer"</h3>
                <p class="subtitle">
                    "Securely deliver files to agents via Ed25519-signed ferry transport"
                </p>
            </div>
            <div class="drop-zone">
                <p>"Drop files here or click to browse"</p>
                <p class="hint">
                    "Files are signed with operator key and transferred via ferry protocol"
                </p>
            </div>
            <div class="relay-section">
                <h3>"Network Relay"</h3>
                <p class="coming-soon">"Relay configuration coming in v3.11"</p>
            </div>
        </div>
    }
}
