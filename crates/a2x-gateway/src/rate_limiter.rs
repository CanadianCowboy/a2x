// Token-bucket rate limiter for the A2X gateway.
// See plans/12-security.md §5 and the comprehensive audit T4-3.
//
// Upgrades the simple fixed-window counter to a proper token-bucket
// algorithm with configurable rate, burst tolerance, and graceful
// refill semantics.

use std::collections::HashMap;
use std::time::Instant;

use crate::entity::EntityId;

/// Individual token bucket for a single entity.
#[derive(Clone, Debug)]
struct TokenBucket {
    /// Maximum number of tokens the bucket can hold.
    capacity: u32,
    /// Current number of tokens available.
    tokens: f64,
    /// Tokens added per second.
    rate: f64,
    /// When the bucket was last refilled.
    last_refill: Instant,
}

impl TokenBucket {
    fn new(capacity: u32, rate_per_sec: f64) -> Self {
        TokenBucket {
            capacity,
            tokens: capacity as f64,
            rate: rate_per_sec,
            last_refill: Instant::now(),
        }
    }

    /// Refill tokens based on elapsed time.
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.rate).min(self.capacity as f64);
        self.last_refill = now;
    }

    /// Try to consume one token. Returns true if allowed.
    fn try_consume(&mut self) -> bool {
        self.refill();
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

/// Token-bucket rate limiter for gateway entities.
///
/// Each entity gets its own bucket. When a request arrives, the
/// bucket is refilled based on elapsed time and one token is consumed.
/// If no tokens are available, the request is rejected.
pub struct RateLimiter {
    /// Per-entity buckets.
    buckets: HashMap<EntityId, TokenBucket>,
}

impl RateLimiter {
    /// Create a new rate limiter.
    /// `rate_per_min` is the allowed requests per minute (used as default bucket capacity).
    pub fn new(_rate_per_min: u32) -> Self {
        RateLimiter {
            buckets: HashMap::new(),
        }
    }

    /// Check if a request from the given entity is allowed.
    /// Returns true if the request should proceed.
    pub fn check(&mut self, entity_id: &EntityId, limit: u32) -> bool {
        let bucket = self.buckets.entry(entity_id.clone()).or_insert_with(|| {
            // Burst capacity = limit, so the entity can burst up to its
            // full minute allowance at once, but must then wait for refill.
            TokenBucket::new(limit, limit as f64 / 60.0)
        });
        bucket.try_consume()
    }

    /// Get the current token count for an entity (for monitoring).
    pub fn tokens(&self, entity_id: &EntityId) -> Option<f64> {
        self.buckets.get(entity_id).map(|b| b.tokens)
    }

    /// Remove all buckets (e.g., on shutdown).
    pub fn clear(&mut self) {
        self.buckets.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn test_token_bucket_allows_up_to_capacity() {
        let mut bucket = TokenBucket::new(5, 1.0);
        for _ in 0..5 {
            assert!(bucket.try_consume());
        }
        assert!(!bucket.try_consume());
    }

    #[test]
    fn test_token_bucket_refills_over_time() {
        let mut bucket = TokenBucket::new(5, 100.0); // 100 tokens/sec
        for _ in 0..5 {
            assert!(bucket.try_consume());
        }
        assert!(!bucket.try_consume());
        sleep(Duration::from_millis(20)); // 0.02s * 100 = 2 tokens
        assert!(bucket.try_consume());
        assert!(bucket.try_consume());
    }

    #[test]
    fn test_rate_limiter_per_entity() {
        let mut limiter = RateLimiter::new(60);
        let e1 = EntityId::new("app-1");
        let e2 = EntityId::new("app-2");

        // Both get their own buckets
        for _ in 0..60 {
            assert!(limiter.check(&e1, 60));
        }
        assert!(!limiter.check(&e1, 60));

        // e2 is unaffected
        assert!(limiter.check(&e2, 60));
    }
}
