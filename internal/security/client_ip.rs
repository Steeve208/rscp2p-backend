use std::net::{IpAddr, SocketAddr};

use axum::http::HeaderMap;

/// Resolve client IP respecting trusted reverse proxies.
pub fn resolve_client_ip(
    peer: Option<SocketAddr>,
    headers: &HeaderMap,
    trusted_proxies: &[String],
) -> Option<String> {
    if let Some(peer) = peer {
        if trusted_proxies.is_empty() || !is_trusted_proxy(peer.ip(), trusted_proxies) {
            return Some(peer.ip().to_string());
        }
    }

    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(str::trim)
        .filter(|ip| !ip.is_empty())
        .map(str::to_string)
        .or_else(|| peer.map(|p| p.ip().to_string()))
}

fn is_trusted_proxy(ip: IpAddr, trusted: &[String]) -> bool {
    trusted.iter().any(|entry| match entry.parse::<IpAddr>() {
        Ok(trusted_ip) => trusted_ip == ip,
        Err(_) => entry == &ip.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;

    #[test]
    fn uses_peer_when_no_trusted_proxies() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "203.0.113.1".parse().unwrap());
        let peer = "127.0.0.1:8080".parse().ok();
        let ip = resolve_client_ip(peer, &headers, &[]);
        assert_eq!(ip.as_deref(), Some("127.0.0.1"));
    }

    #[test]
    fn uses_forwarded_for_when_peer_is_trusted() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "203.0.113.50".parse().unwrap());
        let peer = "127.0.0.1:8080".parse().ok();
        let ip = resolve_client_ip(peer, &headers, &["127.0.0.1".into()]);
        assert_eq!(ip.as_deref(), Some("203.0.113.50"));
    }
}
