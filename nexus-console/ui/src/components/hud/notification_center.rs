//! Toast notifications + notification drawer.

use leptos::prelude::*;

use super::hud_state::HudState;
use super::types::NotificationSeverity;

#[component]
pub fn NotificationCenter(hud: RwSignal<HudState>) -> impl IntoView {
    let unread_count = move || hud.get().unread_count();
    let (drawer_open, set_drawer_open) = signal(false);

    let toasts = move || hud.get().active_toasts.clone();
    let all_notifications = move || hud.get().notifications.clone();

    view! {
        // Toast stack
        <div class="toast-stack">
            {move || toasts().into_iter().map(|toast| {
                let id = toast.id.clone();
                let severity_class = match toast.severity {
                    NotificationSeverity::Info => "toast-info",
                    NotificationSeverity::Warning => "toast-warning",
                    NotificationSeverity::Critical => "toast-critical",
                };
                let title = toast.title.clone();
                let body = toast.body.clone();
                view! {
                    <div class=format!("toast {severity_class}")>
                        <div class="toast-content">
                            <strong>{title}</strong>
                            <p>{body}</p>
                        </div>
                        <button class="toast-dismiss"
                            on:click={
                                let id = id.clone();
                                move |_| hud.update(|s| s.dismiss_toast(&id))
                            }
                        >"×"</button>
                    </div>
                }
            }).collect_view()}
        </div>

        // Bell icon
        <button class="hud-bell" on:click=move |_| set_drawer_open.update(|v| *v = !*v)>
            <span class="bell-icon">"🔔"</span>
            {move || {
                let count = unread_count();
                (count > 0).then(|| view! {
                    <span class="badge">{count}</span>
                })
            }}
        </button>

        // Notification drawer
        {move || drawer_open.get().then(|| {
            let items = all_notifications();
            view! {
                <div class="notification-drawer">
                    <div class="drawer-header">
                        <h3>"Notifications"</h3>
                        <button class="close-btn"
                            on:click=move |_| set_drawer_open.set(false)
                        >"×"</button>
                    </div>
                    <div class="drawer-list">
                        {items.into_iter().map(|n| {
                            let severity_class = match n.severity {
                                NotificationSeverity::Info => "notif-info",
                                NotificationSeverity::Warning => "notif-warning",
                                NotificationSeverity::Critical => "notif-critical",
                            };
                            let read_class = if n.read { "read" } else { "unread" };
                            let title = n.title.clone();
                            let body = n.body.clone();
                            let source = n.source.clone();
                            view! {
                                <div class=format!("notif-item {severity_class} {read_class}")>
                                    <div class="notif-title">{title}</div>
                                    <div class="notif-body">{body}</div>
                                    <div class="notif-source">{source}</div>
                                </div>
                            }
                        }).collect_view()}
                    </div>
                </div>
            }
        })}
    }
}
