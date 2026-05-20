//! Loopback-only enforcement (D-V1-E carry-over).
//!
//! v1.1 keeps the v1.0 gate: refuses to bind/dial non-loopback addresses
//! unless `insecure_network = true`. mTLS hardening lands at v1.2.

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

/// Enforce the gate.
pub fn enforce<A: ToSocketAddrs + std::fmt::Display>(
    addr: A,
    insecure_network: bool,
) -> Result<(), LoopbackError> {
    let display = format!("{addr}");
    match classify(addr) {
        LoopbackCheck::Loopback => Ok(()),
        LoopbackCheck::Network if insecure_network => Ok(()),
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
         (v1.1 has no mTLS; D-V1-E)"
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
        enforce("127.0.0.1:50051", false).expect("loopback accepted");
    }

    #[test]
    fn enforce_routable_rejects_by_default() {
        let err = enforce("8.8.8.8:50051", false).expect_err("must reject");
        assert!(matches!(err, LoopbackError::NonLoopback(_)));
    }

    #[test]
    fn enforce_routable_with_opt_in_accepts() {
        enforce("8.8.8.8:50051", true).expect("opt-in accepted");
    }
}
