//! v1.4 S3 audit-archive sink (Phase 1.4.9 / D-V1.4-G).
//!
//! Uploads rotated `audit.log.N.gz` files to an S3-compatible bucket
//! after `audit_verify` confirms chain integrity. Works against AWS
//! S3, MinIO, Cloudflare R2, Backblaze B2.
//!
//! Behind the `s3` Cargo feature; off by default so deployments that
//! don't use S3 archival don't pull in the AWS SDK.
//!
//! v1.4.0 scaffold: configuration surface area; real upload + retry
//! lands in Phase 1.4.9.

#![allow(dead_code)]

/// Configuration for the S3 archive sink.
#[derive(Debug, Clone)]
pub struct S3SinkOptions {
    /// Bucket name.
    pub bucket: String,
    /// Object key prefix (records land at `<prefix>/<date>/<filename>`).
    pub key_prefix: String,
    /// Optional KMS key ID for server-side encryption.
    pub kms_key_id: Option<String>,
    /// Optional endpoint override for S3-compatible providers
    /// (MinIO, R2, B2). When `None`, uses the AWS SDK's default
    /// resolution.
    pub endpoint_url: Option<String>,
    /// Maximum upload retry attempts.
    pub max_retries: u32,
}

impl Default for S3SinkOptions {
    fn default() -> Self {
        Self {
            bucket: String::new(),
            key_prefix: "nexus-audit".to_string(),
            kms_key_id: None,
            endpoint_url: None,
            max_retries: 5,
        }
    }
}
