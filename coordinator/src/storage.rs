/// Minimal SigV4 presigned GET URL generator for S3-compatible services.
/// Used when a segment's `storage_uri` starts with `s3://` — the coordinator
/// redirects the browser directly to a presigned S3 URL instead of proxying
/// through the node's playback HTTP server.
use chrono::Utc;
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

pub struct S3Config {
    /// e.g. "http://minio:9000" or "https://s3.us-east-1.amazonaws.com"
    pub endpoint:   String,
    pub key_id:     String,
    pub secret_key: String,
    pub region:     String,
    pub path_style: bool,
}

impl S3Config {
    pub fn from_env() -> Option<Self> {
        Some(Self {
            endpoint:   std::env::var("S3_ENDPOINT").ok()?,
            key_id:     std::env::var("S3_KEY_ID").ok()?,
            secret_key: std::env::var("S3_SECRET_KEY").ok()?,
            region:     std::env::var("S3_REGION").unwrap_or_else(|_| "us-east-1".into()),
            path_style: std::env::var("S3_PATH_STYLE")
                .map(|v| v == "1" || v.to_lowercase() == "true")
                .unwrap_or(true),
        })
    }
}

fn hmac_sha256(key: &[u8], msg: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC key length");
    mac.update(msg);
    mac.finalize().into_bytes().to_vec()
}

fn sha256_hex(data: &[u8]) -> String {
    hex::encode(Sha256::digest(data))
}

fn signing_key(secret: &str, date: &str, region: &str) -> Vec<u8> {
    let k_date    = hmac_sha256(format!("AWS4{secret}").as_bytes(), date.as_bytes());
    let k_region  = hmac_sha256(&k_date,   region.as_bytes());
    let k_service = hmac_sha256(&k_region, b"s3");
    hmac_sha256(&k_service, b"aws4_request")
}

/// Percent-encode a query parameter value per RFC 3986 unreserved characters.
/// Slashes are encoded as %2F because this is used for the `X-Amz-Credential`
/// query parameter value, where `/` is a separator in the credential scope.
fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' |
            b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => { out.push('%'); out.push_str(&format!("{b:02X}")); }
        }
    }
    out
}

/// Generate a presigned GET URL valid for `expires_secs` seconds.
///
/// `storage_uri` must be in the form `s3://{bucket}/{key}`.
/// Returns `None` if S3 is not configured or the URI is malformed.
pub fn presigned_get(storage_uri: &str, expires_secs: u64) -> Option<String> {
    let cfg = S3Config::from_env()?;

    // Parse "s3://bucket/key/path"
    let rest   = storage_uri.strip_prefix("s3://")?;
    let slash  = rest.find('/')?;
    let bucket = &rest[..slash];
    let key    = &rest[slash + 1..];

    let now      = Utc::now();
    let datetime = now.format("%Y%m%dT%H%M%SZ").to_string();
    let date     = now.format("%Y%m%d").to_string();

    let endpoint = cfg.endpoint.trim_end_matches('/');
    let (host, canonical_path) = if cfg.path_style {
        let scheme_end = endpoint.find("://").map(|i| i + 3).unwrap_or(0);
        let host_part  = &endpoint[scheme_end..];
        let path       = format!("/{bucket}/{}", key);
        (host_part.to_string(), path)
    } else {
        let scheme_end = endpoint.find("://").map(|i| i + 3).unwrap_or(0);
        let host_part  = format!("{bucket}.{}", &endpoint[scheme_end..]);
        let path       = format!("/{key}");
        (host_part, path)
    };

    let credential_scope = format!("{date}/{}/s3/aws4_request", cfg.region);
    let canonical_qs = format!(
        "X-Amz-Algorithm=AWS4-HMAC-SHA256\
         &X-Amz-Credential={}\
         &X-Amz-Date={datetime}\
         &X-Amz-Expires={expires_secs}\
         &X-Amz-SignedHeaders=host",
        percent_encode(&format!("{}/{credential_scope}", cfg.key_id)),
    );

    let canonical_request = format!(
        "GET\n{canonical_path}\n{canonical_qs}\nhost:{host}\n\nhost\nUNSIGNED-PAYLOAD"
    );

    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{datetime}\n{credential_scope}\n{}",
        sha256_hex(canonical_request.as_bytes())
    );

    let sig_key   = signing_key(&cfg.secret_key, &date, &cfg.region);
    let signature = hex::encode(hmac_sha256(&sig_key, string_to_sign.as_bytes()));

    let scheme = if endpoint.starts_with("https://") { "https" } else { "http" };
    let url = format!(
        "{scheme}://{host}{canonical_path}?{canonical_qs}&X-Amz-Signature={signature}"
    );
    Some(url)
}
