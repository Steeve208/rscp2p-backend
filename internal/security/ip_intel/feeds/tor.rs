//! Tor Project bulk exit list feed.

use std::collections::HashSet;

use reqwest::Client;
use thiserror::Error;

use super::parse_ip_lines;

#[derive(Debug, Error)]
pub enum TorFeedError {
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("feed returned empty exit list")]
    Empty,
}

/// Download and parse the Tor bulk exit list.
pub async fn fetch_tor_exits(client: &Client, url: &str) -> Result<HashSet<String>, TorFeedError> {
    let body = client.get(url).send().await?.error_for_status()?.text().await?;
    let exits = parse_ip_lines(&body);
    if exits.is_empty() {
        return Err(TorFeedError::Empty);
    }
    Ok(exits)
}
