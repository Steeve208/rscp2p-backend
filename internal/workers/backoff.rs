//! Exponential backoff with jitter for worker retries.

use std::time::Duration;

use rand::Rng;

/// Compute delay for attempt `n` (1-indexed after first failure).
///
/// Formula: `min(base * 2^(n-1), max)` + up to 10% jitter.
pub fn exponential_backoff(attempt: u32, base_ms: u64, max_ms: u64) -> Duration {
    let attempt = attempt.max(1);
    let exp = base_ms.saturating_mul(2_u64.saturating_pow(attempt.saturating_sub(1).min(20)));
    let capped = exp.min(max_ms);

    let jitter = if capped > 0 {
        rand::thread_rng().gen_range(0..=(capped / 10).max(1))
    } else {
        0
    };

    Duration::from_millis(capped.saturating_add(jitter))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_grows_exponentially() {
        let d1 = exponential_backoff(1, 1000, 60_000);
        let d2 = exponential_backoff(2, 1000, 60_000);
        let d3 = exponential_backoff(3, 1000, 60_000);

        assert!(d2 > d1);
        assert!(d3 > d2);
    }

    #[test]
    fn backoff_respects_max() {
        let d = exponential_backoff(20, 1000, 5000);
        assert!(d.as_millis() <= 5500); // max + 10% jitter
    }
}
