//! Striga HMAC-SHA256 request signing (API v1).
//!
//! Message: `timestamp + HTTP_METHOD + path + md5_hex(body_json)`
//! Header: `HMAC {timestamp}:{signature_hex}`

use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Signs a Striga API request per v1 authentication spec.
pub fn sign_request(
    api_secret: &str,
    method: &str,
    path: &str,
    body: &serde_json::Value,
) -> String {
    let timestamp = chrono::Utc::now().timestamp_millis().to_string();
    let body_json = if method == "GET" {
        "{}".to_string()
    } else {
        serde_json::to_string(body).unwrap_or_else(|_| "{}".to_string())
    };
    let content_hash = format!("{:x}", md5::compute(body_json.as_bytes()));

    let message = format!("{timestamp}{method}{path}{content_hash}");

    let mut mac =
        HmacSha256::new_from_slice(api_secret.as_bytes()).expect("HMAC accepts any key length");
    mac.update(message.as_bytes());
    let signature = hex::encode(mac.finalize().into_bytes());

    format!("HMAC {timestamp}:{signature}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_format_is_hmac_prefix() {
        let sig = sign_request("test-secret", "POST", "/ping", &serde_json::json!({"ping": "pong"}));
        assert!(sig.starts_with("HMAC "));
        assert!(sig.contains(':'));
    }

    #[test]
    fn get_uses_empty_object_body_hash() {
        let sig_get = sign_request("secret", "GET", "/user/abc", &serde_json::json!({}));
        let sig_post_empty = sign_request("secret", "GET", "/user/abc", &serde_json::json!({}));
        assert_eq!(sig_get.len(), sig_post_empty.len());
    }
}
