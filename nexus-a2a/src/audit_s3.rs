//! v1.4 S3 audit-archive sink (Phase 1.4.9 / D-V1.4-G).
//!
//! Uploads finalized [`AuditRecord`]s to an S3-compatible bucket as
//! JSON-lines objects, *or* uploads pre-rotated `audit.log.N.gz` files
//! after `audit_verify` confirms chain integrity. Works against AWS
//! S3, MinIO, Cloudflare R2, Backblaze B2 (set [`S3SinkOptions::endpoint_url`]
//! to point at the non-AWS endpoint).
//!
//! Behind the `s3` Cargo feature so deployments that don't use S3
//! archival don't pull in the AWS SDK. The feature-off path keeps only
//! [`S3SinkOptions`] (configuration-only) so callers can wire up config
//! parsing without conditional code.
//!
//! ## Authentication
//!
//! When [`S3SinkOptions::static_credentials`] is `Some`, those are used
//! directly. Otherwise the standard AWS credential chain is used:
//! env vars (`AWS_ACCESS_KEY_ID` / `AWS_SECRET_ACCESS_KEY` /
//! `AWS_SESSION_TOKEN`), shared config / credential files, IAM
//! instance/IRSA roles. This matches the AWS SDK's
//! `behavior-version-latest` defaults.

#![allow(dead_code)]

/// Hardcoded static credentials (use only when an external secret store
/// supplies the values at process start; otherwise leave `None` and let
/// the AWS SDK's credential chain resolve them).
#[derive(Debug, Clone)]
pub struct StaticCredentials {
    /// Access key ID.
    pub access_key_id: String,
    /// Secret access key.
    pub secret_access_key: String,
    /// Optional STS session token.
    pub session_token: Option<String>,
}

/// Configuration for the S3 archive sink.
#[derive(Debug, Clone)]
pub struct S3SinkOptions {
    /// Bucket name.
    pub bucket: String,
    /// AWS region. Required even for MinIO / R2 (the SDK uses it as the
    /// signing region; for non-AWS the value is informational —
    /// `us-east-1` is fine).
    pub region: String,
    /// Object key prefix (records land at `<prefix>/<date>/<filename>`).
    pub key_prefix: String,
    /// Optional KMS key ID for server-side encryption.
    pub kms_key_id: Option<String>,
    /// Optional endpoint override for S3-compatible providers
    /// (MinIO, R2, B2). When `None`, uses the AWS SDK's default
    /// resolution. Example: `"https://<account>.r2.cloudflarestorage.com"`.
    pub endpoint_url: Option<String>,
    /// Whether to force path-style addressing (required by MinIO + most
    /// non-AWS providers; AWS itself uses virtual-hosted style).
    pub force_path_style: bool,
    /// Maximum upload retry attempts.
    pub max_retries: u32,
    /// Static credentials override. Leave `None` to use the default
    /// AWS credential chain (recommended).
    pub static_credentials: Option<StaticCredentials>,
}

impl Default for S3SinkOptions {
    fn default() -> Self {
        Self {
            bucket: String::new(),
            region: "us-east-1".to_string(),
            key_prefix: "nexus-audit".to_string(),
            kms_key_id: None,
            endpoint_url: None,
            force_path_style: false,
            max_retries: 5,
            static_credentials: None,
        }
    }
}

impl S3SinkOptions {
    fn new_for_test(bucket: impl Into<String>) -> Self {
        Self {
            bucket: bucket.into(),
            ..Self::default()
        }
    }
}

// ---------------------------------------------------------------------------
// Feature-gated real implementation
// ---------------------------------------------------------------------------

#[cfg(feature = "s3")]
mod imp {
    use super::S3SinkOptions;
    use crate::audit::{AuditRecord, AuditSink};

    use std::path::Path;
    use std::sync::Arc;
    use std::time::Duration;

    use async_trait::async_trait;
    use aws_credential_types::Credentials;
    use aws_sdk_s3::config::{Region, SharedCredentialsProvider};
    use aws_sdk_s3::error::SdkError;
    use aws_sdk_s3::primitives::ByteStream;
    use aws_sdk_s3::types::ServerSideEncryption;
    use aws_sdk_s3::Client;
    use tokio::sync::mpsc;
    use tracing::{debug, warn};

    const DEFAULT_QUEUE_CAPACITY: usize = 1024;
    const INITIAL_BACKOFF: Duration = Duration::from_millis(250);
    const MAX_BACKOFF: Duration = Duration::from_secs(30);

    /// S3 audit-archive sink. Construct via [`S3Sink::connect`], then
    /// wrap with [`MultiSink`](crate::audit::MultiSink) so it sees the
    /// same records as the on-disk `FileSink`.
    ///
    /// Records are pushed onto a bounded in-memory queue and uploaded
    /// by a background task. Network failures retry with exponential
    /// backoff up to [`S3SinkOptions::max_retries`]; after that the
    /// record is logged and dropped (matches the SyslogSink contract).
    pub struct S3Sink {
        tx: mpsc::Sender<AuditRecord>,
    }

    impl S3Sink {
        /// Connect to the bucket, build the S3 client, and spawn the
        /// upload task. Returns immediately; uploads happen
        /// asynchronously in the background.
        pub async fn connect(opts: S3SinkOptions) -> Result<Self, S3SinkError> {
            if opts.bucket.is_empty() {
                return Err(S3SinkError::ConfigMissing("bucket"));
            }
            let client = build_client(&opts).await?;
            let (tx, rx) = mpsc::channel(DEFAULT_QUEUE_CAPACITY);
            tokio::spawn(upload_task(client, opts, rx));
            Ok(Self { tx })
        }

        /// Upload a single rotated audit log file (or any byte blob).
        /// Bypasses the streaming queue — synchronous behaviour suited
        /// to operators driving archival from a cron job after
        /// `audit_verify` confirms chain integrity.
        pub async fn upload_blob(
            opts: &S3SinkOptions,
            object_key: &str,
            body: Vec<u8>,
        ) -> Result<(), S3SinkError> {
            let client = build_client(opts).await?;
            put_object_with_retry(&client, opts, object_key, body).await
        }

        /// Upload a file path (convenience wrapper around
        /// [`S3Sink::upload_blob`]).
        pub async fn upload_file(
            opts: &S3SinkOptions,
            object_key: &str,
            path: &Path,
        ) -> Result<(), S3SinkError> {
            let bytes = tokio::fs::read(path).await?;
            Self::upload_blob(opts, object_key, bytes).await
        }
    }

    #[async_trait]
    impl AuditSink for S3Sink {
        async fn append(&self, record: AuditRecord) {
            if self.tx.try_send(record).is_err() {
                warn!("s3 sink queue full; dropping record");
            }
        }
    }

    async fn build_client(opts: &S3SinkOptions) -> Result<Client, S3SinkError> {
        let mut loader = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(Region::new(opts.region.clone()));

        if let Some(creds) = &opts.static_credentials {
            loader = loader.credentials_provider(SharedCredentialsProvider::new(
                Credentials::from_keys(
                    creds.access_key_id.clone(),
                    creds.secret_access_key.clone(),
                    creds.session_token.clone(),
                ),
            ));
        }
        if let Some(url) = &opts.endpoint_url {
            loader = loader.endpoint_url(url.clone());
        }
        let shared = loader.load().await;

        let mut s3_config = aws_sdk_s3::config::Builder::from(&shared);
        if opts.force_path_style {
            s3_config = s3_config.force_path_style(true);
        }
        Ok(Client::from_conf(s3_config.build()))
    }

    async fn upload_task(client: Client, opts: S3SinkOptions, mut rx: mpsc::Receiver<AuditRecord>) {
        let opts = Arc::new(opts);
        while let Some(record) = rx.recv().await {
            let key = object_key_for_record(&opts.key_prefix, &record);
            let body = match serde_json::to_vec(&record) {
                Ok(b) => b,
                Err(err) => {
                    warn!(error = %err, "s3 sink serialize failed; dropping record");
                    continue;
                }
            };
            if let Err(err) = put_object_with_retry(&client, &opts, &key, body).await {
                warn!(error = %err, key = %key, "s3 sink upload exhausted retries; dropping record");
            }
        }
        debug!("s3 sink upload task ending");
    }

    async fn put_object_with_retry(
        client: &Client,
        opts: &S3SinkOptions,
        object_key: &str,
        body: Vec<u8>,
    ) -> Result<(), S3SinkError> {
        let mut backoff = INITIAL_BACKOFF;
        for attempt in 0..=opts.max_retries {
            let stream = ByteStream::from(body.clone());
            let mut req = client
                .put_object()
                .bucket(&opts.bucket)
                .key(object_key)
                .body(stream)
                .content_type("application/json");
            if let Some(kms) = &opts.kms_key_id {
                req = req
                    .server_side_encryption(ServerSideEncryption::AwsKms)
                    .ssekms_key_id(kms);
            }
            match req.send().await {
                Ok(_) => {
                    debug!(attempt, key = %object_key, "s3 sink: object uploaded");
                    return Ok(());
                }
                Err(err) => {
                    if !is_retryable(&err) || attempt == opts.max_retries {
                        return Err(S3SinkError::Upload(format!("{err}")));
                    }
                    warn!(
                        attempt,
                        key = %object_key,
                        error = %err,
                        "s3 sink: transient upload error; backing off"
                    );
                    tokio::time::sleep(backoff).await;
                    backoff = (backoff * 2).min(MAX_BACKOFF);
                }
            }
        }
        Err(S3SinkError::Upload("retries exhausted".into()))
    }

    fn is_retryable<E, R>(err: &SdkError<E, R>) -> bool {
        matches!(
            err,
            SdkError::DispatchFailure(_)
                | SdkError::TimeoutError(_)
                | SdkError::ResponseError(_)
                | SdkError::ServiceError(_)
        )
    }

    /// `<prefix>/YYYY/MM/DD/<unix>-<record_hash>.json`
    fn object_key_for_record(prefix: &str, record: &AuditRecord) -> String {
        let (y, m, d) = ymd_from_unix(record.timestamp_unix);
        format!(
            "{prefix}/{y:04}/{m:02}/{d:02}/{ts:013}-{rh}.json",
            prefix = prefix.trim_end_matches('/'),
            y = y,
            m = m,
            d = d,
            ts = record.timestamp_unix,
            rh = if record.record_hash.len() >= 16 {
                &record.record_hash[..16]
            } else {
                record.record_hash.as_str()
            }
        )
    }

    fn ymd_from_unix(unix: u64) -> (u32, u32, u32) {
        let days = unix / 86_400;
        let z = days as i64 + 719_468;
        let era = z.div_euclid(146_097);
        let doe = (z - era * 146_097) as u64;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let y = yoe as i64 + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        let y = if m <= 2 { y + 1 } else { y };
        (y as u32, m as u32, d as u32)
    }

    /// Errors emitted by the S3 sink.
    #[derive(Debug, thiserror::Error)]
    pub enum S3SinkError {
        /// Required option missing from [`S3SinkOptions`].
        #[error("s3 sink: missing required option `{0}`")]
        ConfigMissing(&'static str),
        /// Local filesystem error (when uploading a file path).
        #[error("s3 sink io: {0}")]
        Io(#[from] std::io::Error),
        /// All retries against the bucket failed.
        #[error("s3 sink upload: {0}")]
        Upload(String),
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::audit::AuditRecord;

        fn record_at(ts: u64) -> AuditRecord {
            AuditRecord {
                timestamp_unix: ts,
                actor: "op".into(),
                action: "x".into(),
                resource: "y".into(),
                prev_hash: "0".repeat(64),
                record_hash: "ab".repeat(32),
            }
        }

        #[test]
        fn object_key_format_is_date_partitioned() {
            // 2026-05-19 00:00:00 UTC → 1_779_148_800
            let key = object_key_for_record("nexus-audit", &record_at(1_779_148_800));
            // 13-char zero-padded timestamp matches DtnQueue filename style.
            assert!(
                key.starts_with("nexus-audit/2026/05/19/0001779148800-abababab"),
                "got {key}"
            );
            assert!(key.ends_with(".json"));
        }

        #[test]
        fn object_key_strips_trailing_slash_from_prefix() {
            let key = object_key_for_record("nexus-audit/", &record_at(1_779_148_800));
            assert!(!key.starts_with("nexus-audit//"));
            assert!(key.starts_with("nexus-audit/2026/05/19/"));
        }

        #[test]
        fn ymd_round_trip_for_known_dates() {
            // 2023-01-01
            assert_eq!(ymd_from_unix(1_672_531_200), (2023, 1, 1));
            // 2026-05-19
            assert_eq!(ymd_from_unix(1_779_148_800), (2026, 5, 19));
            // Unix epoch
            assert_eq!(ymd_from_unix(0), (1970, 1, 1));
        }

        #[tokio::test]
        async fn connect_fails_without_bucket() {
            let result = S3Sink::connect(S3SinkOptions::default()).await;
            assert!(matches!(result, Err(S3SinkError::ConfigMissing("bucket"))));
        }
    }
}

#[cfg(feature = "s3")]
pub use imp::{S3Sink, S3SinkError};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_options_have_sane_values() {
        let opts = S3SinkOptions::default();
        assert_eq!(opts.region, "us-east-1");
        assert_eq!(opts.key_prefix, "nexus-audit");
        assert_eq!(opts.max_retries, 5);
        assert!(!opts.force_path_style);
        assert!(opts.kms_key_id.is_none());
        assert!(opts.endpoint_url.is_none());
        assert!(opts.static_credentials.is_none());
    }

    #[test]
    fn options_constructor_helper_sets_bucket() {
        let opts = S3SinkOptions::new_for_test("my-bucket");
        assert_eq!(opts.bucket, "my-bucket");
    }
}
