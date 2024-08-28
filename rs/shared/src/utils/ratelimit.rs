use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::time::sleep;

const TOKEN_RESET_INTERVAL: Duration = Duration::from_secs(60);

#[derive(Clone)]
pub struct RateLimiter {
    tokens_used: Arc<Mutex<usize>>,
    last_reset: Arc<Mutex<Instant>>,
    token_limit: usize,
}

impl RateLimiter {
    pub fn new(token_limit: usize) -> Self {
        RateLimiter {
            tokens_used: Arc::new(Mutex::new(0)),
            last_reset: Arc::new(Mutex::new(Instant::now())),
            token_limit,
        }
    }

    pub async fn check_rate_limit(&self, token_count: usize) {
        let mut tokens_used = self.tokens_used.lock().unwrap();
        let mut last_reset = self.last_reset.lock().unwrap();

        if last_reset.elapsed() >= TOKEN_RESET_INTERVAL {
            *tokens_used = 0;
            *last_reset = Instant::now();
        }

        if *tokens_used + token_count > self.token_limit {
            let sleep_duration = TOKEN_RESET_INTERVAL - last_reset.elapsed();
            sleep(sleep_duration).await;
            *tokens_used = 0;
            *last_reset = Instant::now();
        }

        *tokens_used += token_count;
    }
}
