/// Minimal AWS Signature Version 4 implementation for S3 PUT requests.
/// Works with both AWS S3 and any S3-compatible service (MinIO, Backblaze B2, etc.).
use chrono::Utc;
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

type HmacSha256 = Hmac<Sha256>;

pub struct S3Config {
    /// e.g. "http://minio:9000" or "https://s3.amazonaws.com"
    pub endpoint:   String,
    pub bucket:     String,
    pub key_id:     String,
    pub secret_key: String,
    /// e.g. "us-east-1" (use "us-east-1" for MinIO too)
    pub region:     String,
    /// Path-style URLs: `endpoint/{bucket}/{key}`.
    /// Virtual-hosted style: `{bucket}.endpoint/{key}`.
    /// MinIO uses path-style; AWS S3 defaults to virtual-hosted.
    pub path_style: bool,
}

impl S3Config {
    pub fn from_env() -> Option<Self> {
        Some(Self {
            endpoint:   std::env::var("S3_ENDPOINT").ok()?,
            bucket:     std::env::var("S3_BUCKET").ok()?,
            key_id:     std::env::var("S3_KEY_ID").ok()?,
            secret_key: std::env::var("S3_SECRET_KEY").ok()?,
            region:     std::env::var("S3_REGION").unwrap_or_else(|_| "us-east-1".into()),
            path_style: std::env::var("S3_PATH_STYLE")
                .map(|v| v == "1" || v.to_lowercase() == "true")
                .unwrap_or(true),
        })
    }

    /// Build the URL for a given object key.
    pub fn object_url(&self, key: &str) -> String {
        let endpoint = self.endpoint.trim_end_matches('/');
        if self.path_style {
            format!("{}/{}/{}", endpoint, self.bucket, key.trim_start_matches('/'))
        } else {
            // Strip scheme for virtual-hosted rewrite
            let scheme_end = endpoint.find("://").map(|i| i + 3).unwrap_or(0);
            format!(
                "{}{}.{}/{}",
                &endpoint[..scheme_end],
                self.bucket,
                &endpoint[scheme_end..],
                key.trim_start_matches('/')
            )
        }
    }

    /// The hostname used in the `Host` header and canonical request.
    pub fn host(&self) -> String {
        let endpoint = self.endpoint.trim_end_matches('/');
        let scheme_end = endpoint.find("://").map(|i| i + 3).unwrap_or(0);
        let host_part = &endpoint[scheme_end..];
        if self.path_style {
            host_part.to_string()
        } else {
            format!("{}.{}", self.bucket, host_part)
        }
    }
}

/// Compute HMAC-SHA256.
fn hmac_sha256(key: &[u8], msg: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC key length");
    mac.update(msg);
    mac.finalize().into_bytes().to_vec()
}

/// Compute SHA-256 and return lowercase hex.
fn sha256_hex(data: &[u8]) -> String {
    hex::encode(Sha256::digest(data))
}

/// Derive the signing key from credentials and scope.
fn signing_key(secret: &str, date: &str, region: &str) -> Vec<u8> {
    let k_date    = hmac_sha256(format!("AWS4{secret}").as_bytes(), date.as_bytes());
    let k_region  = hmac_sha256(&k_date,   region.as_bytes());
    let k_service = hmac_sha256(&k_region, b"s3");
    hmac_sha256(&k_service, b"aws4_request")
}

/// Returns the signed headers needed for a `PUT {url}` request.
/// The caller must include all returned headers in the `reqwest` request.
pub fn signed_put_headers(
    cfg:  &S3Config,
    key:  &str,
    body: &[u8],
) -> BTreeMap<String, String> {
    let now      = Utc::now();
    let datetime = now.format("%Y%m%dT%H%M%SZ").to_string();
    let date     = now.format("%Y%m%d").to_string();

    let host          = cfg.host();
    let payload_hash  = sha256_hex(body);
    let content_type  = "video/mp4";

    // Canonical path: path-style = /{bucket}/{key}; virtual = /{key}
    let canonical_path = if cfg.path_style {
        format!("/{}/{}", cfg.bucket, key.trim_start_matches('/'))
    } else {
        format!("/{}", key.trim_start_matches('/'))
    };

    // Sorted canonical headers (must be lowercase, trimmed)
    let canonical_headers = format!(
        "content-type:{content_type}\nhost:{host}\nx-amz-content-sha256:{payload_hash}\nx-amz-date:{datetime}\n"
    );
    let signed_headers = "content-type;host;x-amz-content-sha256;x-amz-date";

    let canonical_request = format!(
        "PUT\n{canonical_path}\n\n{canonical_headers}\n{signed_headers}\n{payload_hash}"
    );

    let credential_scope = format!("{date}/{}/{}/aws4_request", cfg.region, "s3");
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{datetime}\n{credential_scope}\n{}",
        sha256_hex(canonical_request.as_bytes())
    );

    let sig_key   = signing_key(&cfg.secret_key, &date, &cfg.region);
    let signature = hex::encode(hmac_sha256(&sig_key, string_to_sign.as_bytes()));

    let authorization = format!(
        "AWS4-HMAC-SHA256 Credential={}/{credential_scope}, SignedHeaders={signed_headers}, Signature={signature}",
        cfg.key_id
    );

    let mut headers = BTreeMap::new();
    headers.insert("host".into(),                    host);
    headers.insert("content-type".into(),            content_type.into());
    headers.insert("x-amz-date".into(),              datetime);
    headers.insert("x-amz-content-sha256".into(),    payload_hash);
    headers.insert("authorization".into(),            authorization);
    headers
}
