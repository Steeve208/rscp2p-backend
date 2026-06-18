//! IP reputation and classification.
//!
//! Production threat-intel feeds:
//! - **Tor exits**: [Tor bulk exit list](https://check.torproject.org/torbulkexitlist)
//! - **Datacenter/hosting**: configurable CIDR netset (default: FireHOL datacenter.netset)
//!
//! Feeds are refreshed in the background by [`refresher::spawn_refresher`] and cached in
//! Redis + memory via [`ThreatIntelStore`].

pub mod blocklist;
pub mod feeds;
pub mod refresher;
pub mod store;

pub use blocklist::IpBlocklist;
pub use store::ThreatIntelStore;

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// Classification of a client IP address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpClass {
    /// Private / loopback / link-local — internal traffic.
    Private,
    /// Routable public address — no additional signals.
    Public,
    /// Known Tor exit node.
    TorExitNode,
    /// Datacenter / hosting / cloud (high-risk for fraud).
    Datacenter,
}

impl IpClass {
    pub fn is_suspicious(self) -> bool {
        matches!(self, Self::TorExitNode | Self::Datacenter)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Private => "private",
            Self::Public => "public",
            Self::TorExitNode => "tor_exit_node",
            Self::Datacenter => "datacenter",
        }
    }
}

/// Synchronous classify — private-range check only.
///
/// For Tor/datacenter detection use [`ThreatIntelStore::classify`].
pub fn classify_private_only(ip: &str) -> IpClass {
    let Ok(addr) = ip.parse::<IpAddr>() else {
        return IpClass::Public;
    };
    if is_private(&addr) {
        IpClass::Private
    } else {
        IpClass::Public
    }
}

/// Returns `true` for RFC1918, loopback, link-local, documentation, and CGNAT ranges.
pub fn is_private(addr: &IpAddr) -> bool {
    match addr {
        IpAddr::V4(ip) => is_private_v4(ip),
        IpAddr::V6(ip) => is_private_v6(ip),
    }
}

fn is_private_v4(ip: &Ipv4Addr) -> bool {
    let o = ip.octets();
    ip.is_loopback()
        || ip.is_private()
        || ip.is_link_local()
        || ip.is_unspecified()
        || (o[0] == 100 && (o[1] & 0xC0) == 64)
        || (o[0] == 192 && o[1] == 0 && o[2] == 2)
        || (o[0] == 198 && o[1] == 51 && o[2] == 100)
        || (o[0] == 203 && o[1] == 0 && o[2] == 113)
}

fn is_private_v6(ip: &Ipv6Addr) -> bool {
    ip.is_loopback()
        || ip.is_unspecified()
        || (ip.segments()[0] & 0xFE00 == 0xFC00)
        || (ip.segments()[0] & 0xFFC0 == 0xFE80)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loopback_is_private() {
        assert!(is_private(&"127.0.0.1".parse().unwrap()));
        assert!(is_private(&"::1".parse().unwrap()));
    }

    #[test]
    fn rfc1918_is_private() {
        assert!(is_private(&"10.0.0.1".parse().unwrap()));
        assert!(is_private(&"192.168.1.1".parse().unwrap()));
        assert!(is_private(&"172.16.0.1".parse().unwrap()));
    }

    #[test]
    fn public_ip_is_not_private() {
        assert!(!is_private(&"8.8.8.8".parse().unwrap()));
    }
}
