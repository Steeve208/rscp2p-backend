//! In-memory IP blocklist (complement with Redis `security:blocklist:ip:*`).

use std::collections::HashSet;

/// Thread-safe in-memory IP blocklist.
///
/// In production, complement with a Redis lookup:
/// `GET security:blocklist:ip:<ip>`
#[derive(Debug, Default, Clone)]
pub struct IpBlocklist {
    blocked: HashSet<String>,
}

impl IpBlocklist {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn block(&mut self, ip: impl Into<String>) {
        self.blocked.insert(ip.into());
    }

    pub fn unblock(&mut self, ip: &str) {
        self.blocked.remove(ip);
    }

    pub fn is_blocked(&self, ip: &str) -> bool {
        self.blocked.contains(ip)
    }

    pub fn len(&self) -> usize {
        self.blocked.len()
    }

    pub fn is_empty(&self) -> bool {
        self.blocked.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocklist_works() {
        let mut bl = IpBlocklist::new();
        assert!(!bl.is_blocked("1.2.3.4"));
        bl.block("1.2.3.4");
        assert!(bl.is_blocked("1.2.3.4"));
        bl.unblock("1.2.3.4");
        assert!(!bl.is_blocked("1.2.3.4"));
    }
}
