//! Loopback-only enforcement (D-V1-E carry-over, v1.2 mTLS-aware).
//!
//! Refuses to bind/dial non-loopback addresses unless one of:
//!  - `insecure_network = true` (explicit opt-in), or
//!  - `tls_configured = true`  (mTLS material loaded — D-V1-E reversal).

use std::net::{IpAddr, SocketAddr, ToSocketAddrs};

/// Classification of an address vs. the loopback gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopbackCheck {
    /// Address resolves entirely to loopback.
    Loopback,
    /// Address resolves to one or more non-loopback IPs.
    Network,
    /// Address could not be resolved.
    Unresolvable,
}

/// Classify the supplied address.
#[must_use]
pub fn classify<A: ToSocketAddrs>(addr: A) -> LoopbackCheck {
    let Ok(iter) = addr.to_socket_addrs() else {
        return LoopbackCheck::Unresolvable;
    };
    let resolved: Vec<SocketAddr> = iter.collect();
    if resolved.is_empty() {
        return LoopbackCheck::Unresolvable;
    }
    if resolved.iter().all(|s| s.ip().is_loopback()) {
        LoopbackCheck::Loopback
    } else {
        LoopbackCheck::Network
    }
}

/// Enforce the gate. Non-loopback bind requires either `insecure_network`
/// (explicit opt-in) or `tls_configured` (v1.2 mTLS material loaded).
pub fn enforce<A: ToSocketAddrs + std::fmt::Display>(
    addr: A,
    insecure_network: bool,
    tls_configured: bool,
) -> Result<(), LoopbackError> {
    let display = format!("{addr}");
    match classify(addr) {
        LoopbackCheck::Loopback => Ok(()),
        LoopbackCheck::Network if insecure_network || tls_configured => Ok(()),
        LoopbackCheck::Network => Err(LoopbackError::NonLoopback(display)),
        LoopbackCheck::Unresolvable => Err(LoopbackError::Unresolvable(display)),
    }
}

/// Gate-rejection reasons.
#[derive(Debug, thiserror::Error)]
pub enum LoopbackError {
    /// Address resolved non-loopback without opt-in.
    #[error(
        "refusing to use non-loopback address {0} without --insecure-network \
         and without mTLS configured (D-V1-E)"
    )]
    NonLoopback(String),
    /// Address unresolvable.
    #[error("address {0} could not be resolved")]
    Unresolvable(String),
}

/// True if `ip` is a loopback IP.
#[must_use]
pub fn ip_is_loopback(ip: IpAddr) -> bool {
    ip.is_loopback()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_loopback_v4() {
        assert_eq!(classify("127.0.0.1:1234"), LoopbackCheck::Loopback);
    }

    #[test]
    fn classify_routable_v4() {
        assert_eq!(classify("8.8.8.8:1234"), LoopbackCheck::Network);
    }

    #[test]
    fn enforce_loopback_accepts() {
        enforce("127.0.0.1:50051", false, false).expect("loopback accepted");
    }

    #[test]
    fn enforce_routable_rejects_by_default() {
        let err = enforce("8.8.8.8:50051", false, false).expect_err("must reject");
        assert!(matches!(err, LoopbackError::NonLoopback(_)));
    }

    #[test]
    fn enforce_routable_with_opt_in_accepts() {
        enforce("8.8.8.8:50051", true, false).expect("insecure_network accepted");
    }

    #[test]
    fn enforce_routable_with_tls_accepts() {
        enforce("8.8.8.8:50051", false, true).expect("tls_configured accepted");
    }
}
