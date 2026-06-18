//! Datacenter / hosting CIDR feed (FireHOL netset or custom URL).

use ipnet::IpNet;
use reqwest::Client;
use thiserror::Error;

use super::parse_cidr_lines;

#[derive(Debug, Error)]
pub enum DatacenterFeedError {
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("feed returned no parseable CIDR ranges")]
    Empty,
}

/// Download and parse a datacenter/hosting CIDR netset.
pub async fn fetch_datacenter_nets(
    client: &Client,
    url: &str,
) -> Result<Vec<IpNet>, DatacenterFeedError> {
    let body = client.get(url).send().await?.error_for_status()?.text().await?;
    let nets = parse_cidr_lines(&body);
    if nets.is_empty() {
        return Err(DatacenterFeedError::Empty);
    }
    Ok(nets)
}
