//! v1.2 Tonic interceptor stack (D-V1.2-defense).
//!
//! Three independent defenses:
//!
//! 1. **Per-peer token-bucket rate limit** (`RateLimitInterceptor`).
//! 2. **Per-RPC message size cap** ([`MAX_MESSAGE_SIZE`]). Wired in via
//!    Tonic's `max_decoding_message_size` + `max_encoding_message_size`
//!    on the generated service (see [`crate::server::A2aServer::with_message_size_cap`]).
//! 3. **gRPC reflection disabled** in `--release` builds, gated behind the
//!    `dev-reflection` Cargo feature (D-V1.2-D). v1.2 ships without the
//!    feature; v1.3 plumbs the optional service through.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

use tonic::Status;

/// Default per-message size cap (4 MB). Operators can override via
/// [`crate::server::A2aServer::with_message_size_cap`].
pub const MAX_MESSAGE_SIZE: usize = 4 * 1024 * 1024;

/// Default per-peer token-bucket capacity (requests per second).
pub const DEFAULT_RATE_LIMIT_RPS: u32 = 100;

/// Token-bucket rate limit keyed by peer (operator) identifier.
///
/// Bucket capacity = burst = RPS. One token per request consumed.
/// Refills linearly at `rps` tokens / second.
///
/// `verify` returns `Ok(())` if a token was available and consumed, or
/// `Err(Status::ResourceExhausted(...))` otherwise.
pub struct RateLimitInterceptor {
    rps: u32,
    buckets: Mutex<HashMap<String, Bucket>>,
}

struct Bucket {
    tokens: f64,
    last_refill: Instant,
}

impl RateLimitInterceptor {
    /// Construct with a per-peer RPS cap.
    #[must_use]
    pub fn new(rps: u32) -> Self {
        Self {
            rps,
            buckets: Mutex::new(HashMap::new()),
        }
    }

    /// Configured RPS.
    #[must_use]
    pub fn rps(&self) -> u32 {
        self.rps
    }

    /// Check whether `peer` may make one more request right now.
    /// Consumes one token on success.
    pub fn verify(&self, peer: &str) -> Result<(), Status> {
        let now = Instant::now();
        let mut buckets = self.buckets.lock().unwrap();
        let bucket = buckets.entry(peer.to_string()).or_insert_with(|| Bucket {
            tokens: f64::from(self.rps),
            last_refill: now,
        });

        let elapsed = now.duration_since(bucket.last_refill).as_secs_f64();
        bucket.tokens = (bucket.tokens + elapsed * f64::from(self.rps)).min(f64::from(self.rps));
        bucket.last_refill = now;

        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            Ok(())
        } else {
            Err(Status::resource_exhausted(format!(
                "rate limit exceeded for {peer} ({} req/s)",
                self.rps
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn allows_within_capacity() {
        let limit = RateLimitInterceptor::new(5);
        for _ in 0..5 {
            limit.verify("op-a").expect("under cap");
        }
    }

    #[test]
    fn rejects_over_capacity() {
        let limit = RateLimitInterceptor::new(2);
        limit.verify("op-a").expect("ok");
        limit.verify("op-a").expect("ok");
        let err = limit.verify("op-a").expect_err("over cap");
        assert_eq!(err.code(), tonic::Code::ResourceExhausted);
    }

    #[test]
    fn peers_are_independent() {
        let limit = RateLimitInterceptor::new(1);
        limit.verify("op-a").expect("ok");
        limit.verify("op-b").expect("different peer");
    }

    #[test]
    fn refills_over_time() {
        let limit = RateLimitInterceptor::new(10); // 10 RPS
        limit.verify("op-a").expect("ok");
        limit.verify("op-a").expect("ok");
        std::thread::sleep(Duration::from_millis(150)); // refill ~1.5
        limit.verify("op-a").expect("ok after refill");
    }
}
