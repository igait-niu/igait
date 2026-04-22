//! Transient-failure retry helper with exponential backoff + jitter.
//!
//! The pipeline's correctness rests on a handful of writes that, if lost to
//! a transient 5xx or socket hiccup, leave a job stuck in limbo. This helper
//! wraps such operations with bounded retries so a blip in the Firebase REST
//! endpoint no longer becomes a work-ticket.
//!
//! Classifier philosophy: retry only on errors that are *plausibly* caused by
//! the network or the remote being temporarily unavailable. Permanent
//! failures (400s, auth, body decode) must surface immediately so the caller
//! can react — retrying them just burns time and smears real bugs.

use anyhow::Result;
use std::time::Duration;

/// Policy parameters for `retry_transient`.
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Total attempts including the first. `1` disables retry entirely.
    pub max_attempts: u32,
    /// Base delay for the first retry. Doubles each subsequent attempt,
    /// capped at `max_backoff`.
    pub initial_backoff: Duration,
    /// Upper bound on backoff per attempt.
    pub max_backoff: Duration,
    /// Random jitter added to each backoff to prevent thundering-herd retries
    /// when many workers hit the same transient fault simultaneously.
    pub jitter: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(5),
            jitter: Duration::from_millis(50),
        }
    }
}

impl RetryPolicy {
    /// A shorter policy for latency-sensitive handler paths (e.g., HTTP
    /// request handlers where we'd rather fail fast than make the caller wait).
    pub fn fast() -> Self {
        Self {
            max_attempts: 3,
            initial_backoff: Duration::from_millis(50),
            max_backoff: Duration::from_millis(500),
            jitter: Duration::from_millis(25),
        }
    }
}

/// Runs `op` with exponential-backoff retry on transient failures.
///
/// `op_name` is used only in the attempt-log line so operators can grep for
/// which write is flapping. Pass something descriptive like
/// `"apply_completion_transition"`, not `"write"`.
pub async fn retry_transient<F, Fut, T>(policy: &RetryPolicy, op_name: &str, mut op: F) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut attempt: u32 = 0;
    let mut backoff = policy.initial_backoff;

    loop {
        attempt += 1;
        match op().await {
            Ok(v) => return Ok(v),
            Err(e) => {
                let last = attempt >= policy.max_attempts;
                if last || !is_transient_error(&e) {
                    return Err(e);
                }

                let sleep_for = backoff.saturating_add(pseudo_jitter(policy.jitter));
                eprintln!(
                    "retry_transient[{}] attempt {}/{} failed: {} — retrying in {:?}",
                    op_name, attempt, policy.max_attempts, e, sleep_for
                );
                tokio::time::sleep(sleep_for).await;

                backoff = (backoff * 2).min(policy.max_backoff);
            }
        }
    }
}

/// Classifies an error as transient (safe to retry) or permanent.
///
/// Returns `true` for:
/// - reqwest network/timeout/connect errors
/// - our own `anyhow::bail!` strings that encode a 5xx or 429 status
fn is_transient_error(err: &anyhow::Error) -> bool {
    if let Some(re) = err.downcast_ref::<reqwest::Error>() {
        if re.is_timeout() || re.is_connect() || re.is_body() {
            return true;
        }
        if let Some(status) = re.status() {
            let code = status.as_u16();
            if code == 429 || (500..=599).contains(&code) {
                return true;
            }
        }
    }

    // FirebaseRtdb::{get,set,update,delete,multi_update,...} surface HTTP
    // status failures as `anyhow::bail!("Firebase X failed ({}): {}", status, body)`.
    // Parse the formatted string to recover the status class.
    let msg = format!("{:#}", err);
    if msg.contains("Firebase") && msg.contains("failed (") {
        if msg.contains("failed (5") || msg.contains("failed (429)") {
            return true;
        }
    }

    false
}

/// Cheap pseudo-random jitter derived from the current monotonic clock.
///
/// Not cryptographically random — just enough entropy to spread out
/// simultaneous retries from multiple workers. Using the clock avoids
/// pulling in the `rand` crate for a single use.
fn pseudo_jitter(max: Duration) -> Duration {
    if max.is_zero() {
        return Duration::ZERO;
    }
    let nanos = std::time::Instant::now().elapsed().subsec_nanos() as u128;
    let range = max.as_nanos().max(1);
    Duration::from_nanos((nanos % range) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn returns_first_success_without_sleeping() {
        let calls = Arc::new(AtomicU32::new(0));
        let calls_c = calls.clone();
        let result: Result<i32> = retry_transient(&RetryPolicy::fast(), "test", || {
            let calls = calls_c.clone();
            async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok(42)
            }
        })
        .await;
        assert_eq!(result.unwrap(), 42);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn non_transient_does_not_retry() {
        let calls = Arc::new(AtomicU32::new(0));
        let calls_c = calls.clone();
        let result: Result<()> = retry_transient(&RetryPolicy::fast(), "test", || {
            let calls = calls_c.clone();
            async move {
                calls.fetch_add(1, Ordering::SeqCst);
                anyhow::bail!("permanent bad request (400)")
            }
        })
        .await;
        assert!(result.is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn transient_retries_then_succeeds() {
        let calls = Arc::new(AtomicU32::new(0));
        let calls_c = calls.clone();
        let result: Result<&'static str> = retry_transient(
            &RetryPolicy {
                max_attempts: 4,
                initial_backoff: Duration::from_millis(1),
                max_backoff: Duration::from_millis(2),
                jitter: Duration::ZERO,
            },
            "test",
            || {
                let calls = calls_c.clone();
                async move {
                    let n = calls.fetch_add(1, Ordering::SeqCst);
                    if n < 2 {
                        anyhow::bail!("Firebase PATCH failed (503): upstream unavailable")
                    } else {
                        Ok("ok")
                    }
                }
            },
        )
        .await;
        assert_eq!(result.unwrap(), "ok");
        assert_eq!(calls.load(Ordering::SeqCst), 3);
    }
}
