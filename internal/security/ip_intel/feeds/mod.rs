//! Threat-intel feed downloaders and parsers.

pub mod datacenter;
pub mod tor;

pub use datacenter::fetch_datacenter_nets;
pub use tor::fetch_tor_exits;

use std::collections::HashSet;
use std::net::IpAddr;

use ipnet::IpNet;

/// Parse a newline-delimited list of IP addresses (Tor exit list format).
pub fn parse_ip_lines(body: &str) -> HashSet<String> {
    body.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter(|line| line.parse::<IpAddr>().is_ok())
        .map(str::to_string)
        .collect()
}

/// Parse a newline-delimited list of CIDR ranges or single IPs (FireHOL netset format).
pub fn parse_cidr_lines(body: &str) -> Vec<IpNet> {
    let mut nets = Vec::new();
    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Ok(net) = line.parse::<IpNet>() {
            nets.push(net);
        } else if let Ok(ip) = line.parse::<IpAddr>() {
            if let Ok(net) = IpNet::new(ip, if ip.is_ipv4() { 32 } else { 128 }) {
                nets.push(net);
            }
        }
    }
    nets
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tor_lines_skips_comments() {
        let body = "# comment\n1.2.3.4\n\n5.6.7.8\n";
        let ips = parse_ip_lines(body);
        assert_eq!(ips.len(), 2);
        assert!(ips.contains("1.2.3.4"));
    }

    #[test]
    fn parse_cidr_lines_accepts_single_ips() {
        let nets = parse_cidr_lines("10.0.0.0/8\n203.0.113.10\n");
        assert_eq!(nets.len(), 2);
        let ip: std::net::IpAddr = "10.1.2.3".parse().unwrap();
        assert!(nets[0].contains(&ip));
    }
}
