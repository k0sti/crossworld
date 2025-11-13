use crate::protocol::{AuthLevel, EditOperation};
use cube::CubeCoord;
use rand::{rngs::OsRng, RngCore};
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct ClientSession {
    pub session_id: u64,
    pub npub: String,
    pub auth_level: AuthLevel,
    subscriptions: RwLock<Vec<Subscription>>,
    rate_limiter: RwLock<RateLimiter>,
}

impl ClientSession {
    pub fn new(session_id: u64, npub: String, auth_level: AuthLevel) -> Self {
        Self {
            session_id,
            npub,
            auth_level,
            subscriptions: RwLock::new(Vec::new()),
            rate_limiter: RwLock::new(RateLimiter::new()),
        }
    }

    pub fn can_edit(&self) -> bool {
        matches!(self.auth_level, AuthLevel::User | AuthLevel::Admin)
    }

    pub async fn add_subscription(&self, coord: CubeCoord) -> u64 {
        let mut subs = self.subscriptions.write().await;
        let id = OsRng.next_u64();
        subs.push(Subscription { id, coord });
        id
    }

    pub async fn is_subscribed(&self, coord: &CubeCoord) -> bool {
        self.subscription_for(coord).await.is_some()
    }

    pub async fn subscription_for(&self, coord: &CubeCoord) -> Option<u64> {
        let subs = self.subscriptions.read().await;
        subs.iter()
            .find(|sub| sub.coord.depth == coord.depth && sub.coord.pos == coord.pos)
            .map(|sub| sub.id)
    }

    pub async fn subscription_for_operation(&self, op: &EditOperation) -> Option<u64> {
        match op {
            EditOperation::SetCube { coord, .. } => self.subscription_for(coord).await,
        }
    }

    pub async fn check_rate_limit(&self) -> bool {
        let mut limiter = self.rate_limiter.write().await;
        limiter.check_and_consume()
    }
}

#[derive(Debug, Clone)]
pub struct Subscription {
    pub id: u64,
    pub coord: CubeCoord,
}

#[derive(Debug)]
pub struct RateLimiter {
    tokens: f64,
    max_tokens: f64,
    refill_rate: f64,
    last_refill: std::time::Instant,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            tokens: 10.0,
            max_tokens: 10.0,
            refill_rate: 10.0,
            last_refill: std::time::Instant::now(),
        }
    }

    pub fn check_and_consume(&mut self) -> bool {
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.max_tokens);
        self.last_refill = now;

        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}
