use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use governor::{
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter,
};
use std::num::NonZeroU32;
use std::sync::Arc;

// Define the type of our rate limiter
pub type SharedRateLimiter = Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>;

pub async fn rate_limit_middleware(
    State(limiter): State<SharedRateLimiter>,
    request: Request,
    next: Next,
) -> Response {
    match limiter.check() {
        Ok(_) => next.run(request).await,
        Err(_) => (StatusCode::TOO_MANY_REQUESTS, "Rate limit exceeded").into_response(),
    }
}

pub fn create_limiter(requests_per_second: u32, burst_size: u32) -> SharedRateLimiter {
    let quota = Quota::per_second(NonZeroU32::new(requests_per_second).unwrap_or(NonZeroU32::new(1).unwrap()))
        .allow_burst(NonZeroU32::new(burst_size).unwrap_or(NonZeroU32::new(1).unwrap()));
    Arc::new(RateLimiter::direct(quota))
}