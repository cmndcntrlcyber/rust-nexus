//! Mesh tab — bridge mount point for the React/three.js mesh visualization
//! from v3.10.1. The React bundle attaches to the `#mesh-viz-root` div.

use leptos::prelude::*;

#[component]
pub fn MeshTab() -> impl IntoView {
    view! {
        <div class="tab-content mesh-tab">
            <div id="mesh-viz-root" class="mesh-mount-point">
                <div class="tab-placeholder">
                    <p>"3D Mesh Visualization"</p>
                    <p class="hint">"v3.10.1 — React/three.js component mounts here"</p>
                </div>
            </div>
        </div>
    }
}
