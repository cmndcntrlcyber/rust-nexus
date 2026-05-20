//! v1.3 Prometheus metrics (D-V1.3-C / Phase 1.3.6).
//!
//! Pull-based exposition. Counters + gauges registered against a
//! process-wide `Registry`; the metrics HTTP server in
//! `nexus-infra::metrics_server` serves the text format on a separate
//! port (default `:9100`).

use std::sync::OnceLock;

use prometheus::{
    register_int_counter_vec_with_registry, register_int_gauge_with_registry, IntCounterVec,
    IntGauge, Registry, TextEncoder,
};

/// Default port for the `/metrics` endpoint.
pub const DEFAULT_METRICS_PORT: u16 = 9100;

/// Process-wide metric registry. Initialised on first use via
/// [`registry`].
fn registry() -> &'static Registry {
    static REGISTRY: OnceLock<Registry> = OnceLock::new();
    REGISTRY.get_or_init(Registry::new)
}

/// Top-level metric handles. Cheap clones (`Arc` under the hood).
pub struct Metrics {
    /// `nexus_a2a_requests_total{rpc="…"}` — incremented per inbound RPC.
    pub requests_total: IntCounterVec,
    /// `nexus_a2a_rate_limit_exhausted_total{peer="…"}`.
    pub rate_limit_exhausted_total: IntCounterVec,
    /// `nexus_a2a_capability_denied_total{operator="…",agent="…",skill="…"}`.
    pub capability_denied_total: IntCounterVec,
    /// `nexus_a2a_audit_records_total{action="…"}`.
    pub audit_records_total: IntCounterVec,
    /// `nexus_a2a_active_agent_sessions` (gauge).
    pub active_agent_sessions: IntGauge,
}

impl Metrics {
    /// Get (or initialise) the process-wide `Metrics` instance.
    pub fn global() -> &'static Self {
        static M: OnceLock<Metrics> = OnceLock::new();
        M.get_or_init(|| {
            let reg = registry();
            let requests_total = register_int_counter_vec_with_registry!(
                "nexus_a2a_requests_total",
                "Total RPCs handled, labelled by rpc method.",
                &["rpc"],
                reg
            )
            .expect("register requests_total");
            let rate_limit_exhausted_total = register_int_counter_vec_with_registry!(
                "nexus_a2a_rate_limit_exhausted_total",
                "Requests rejected by the per-peer token bucket.",
                &["peer"],
                reg
            )
            .expect("register rate_limit_exhausted_total");
            let capability_denied_total = register_int_counter_vec_with_registry!(
                "nexus_a2a_capability_denied_total",
                "Operator → agent → skill triples rejected by the capability gate.",
                &["operator", "agent", "skill"],
                reg
            )
            .expect("register capability_denied_total");
            let audit_records_total = register_int_counter_vec_with_registry!(
                "nexus_a2a_audit_records_total",
                "Audit records emitted, labelled by action.",
                &["action"],
                reg
            )
            .expect("register audit_records_total");
            let active_agent_sessions = register_int_gauge_with_registry!(
                "nexus_a2a_active_agent_sessions",
                "Currently-registered A2A-mode agents.",
                reg
            )
            .expect("register active_agent_sessions");
            Metrics {
                requests_total,
                rate_limit_exhausted_total,
                capability_denied_total,
                audit_records_total,
                active_agent_sessions,
            }
        })
    }
}

/// Encode every registered metric in Prometheus text exposition format.
/// The metrics HTTP server returns this body on `/metrics`.
pub fn gather_text() -> Result<String, prometheus::Error> {
    let encoder = TextEncoder::new();
    encoder.encode_to_string(&registry().gather())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counters_register_and_increment() {
        let m = Metrics::global();
        m.requests_total.with_label_values(&["GetAgentCard"]).inc();
        m.rate_limit_exhausted_total
            .with_label_values(&["operator-alice"])
            .inc();
        m.capability_denied_total
            .with_label_values(&["operator-alice", "ab12", "shell-session"])
            .inc();
        m.audit_records_total
            .with_label_values(&["shell_session_open"])
            .inc_by(3);
        m.active_agent_sessions.set(7);

        let text = gather_text().expect("gather");
        assert!(text.contains("nexus_a2a_requests_total"));
        assert!(text.contains("rpc=\"GetAgentCard\""));
        assert!(text.contains("nexus_a2a_active_agent_sessions 7"));
        assert!(text.contains("nexus_a2a_audit_records_total"));
    }

    #[test]
    fn idempotent_global() {
        let a = Metrics::global() as *const _;
        let b = Metrics::global() as *const _;
        assert_eq!(a, b, "global() returns the same instance");
    }
}
