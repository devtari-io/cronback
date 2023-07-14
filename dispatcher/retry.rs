use std::time::Duration;

use futures::future::BoxFuture;
use lib::types::{ExponentialBackoffRetry, RetryConfig, SimpleRetry};

pub struct RetryPolicy {
    config: Option<RetryConfig>,
    retries_limit: u32,
}

impl RetryPolicy {
    pub fn with_config(c: RetryConfig) -> Self {
        let retries_limit = match c {
            | RetryConfig::SimpleRetry(SimpleRetry {
                max_num_attempts,
                ..
            }) => max_num_attempts,
            | RetryConfig::ExponentialBackoffRetry(
                ExponentialBackoffRetry {
                    max_num_attempts, ..
                },
            ) => max_num_attempts,
        };
        Self {
            config: Some(c),
            retries_limit,
        }
    }

    pub fn no_retry() -> Self {
        Self {
            config: None,
            retries_limit: 0,
        }
    }

    // Given an async function F, this function will retry it
    // according to the retry policy as long as F returns Err.
    // If retries are exhausted, the last error from F is returned.
    pub async fn retry<'a, T, U>(
        &self,
        mut f: impl FnMut(u32) -> BoxFuture<'a, Result<T, U>>,
    ) -> Result<T, U> {
        let mut num_retries: u32 = 0;
        loop {
            let result = f(num_retries).await;

            match result {
                | Ok(r) => return Ok(r),
                | Err(e) => {
                    num_retries += 1;
                    match self.next_sleep_duration(num_retries) {
                        | Some(d) => {
                            let jitter = Duration::from_millis(
                                (rand::random::<u16>() % 1000).into(),
                            );
                            tokio::time::sleep(d + jitter).await;
                            continue;
                        }
                        | None => return Err(e),
                    }
                }
            }
        }
    }

    /// Returns the duration to sleep if a retry should be done
    /// or None if we should no longer retry
    fn next_sleep_duration(&self, retry_num: u32) -> Option<Duration> {
        if retry_num > self.retries_limit {
            return None;
        }

        match self.config {
            | Some(RetryConfig::SimpleRetry(SimpleRetry {
                delay_s, ..
            })) => Some(delay_s),
            | Some(RetryConfig::ExponentialBackoffRetry(
                ExponentialBackoffRetry {
                    delay_s,
                    max_delay_s,
                    ..
                },
            )) => {
                Some(std::cmp::min(
                    max_delay_s,
                    delay_s * 2_u32.pow(retry_num - 1),
                ))
            }
            | None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use lib::types::{ExponentialBackoffRetry, RetryConfig, SimpleRetry};

    use super::RetryPolicy;

    #[test]
    fn no_retry_policy() {
        let retry_policy = RetryPolicy::no_retry();

        assert_eq!(retry_policy.next_sleep_duration(1), None);
    }

    #[test]
    fn simple_retry_policy_delays() {
        let config = RetryConfig::SimpleRetry(SimpleRetry {
            max_num_attempts: 3,
            delay_s: Duration::from_secs(100),
        });

        let retry_policy = RetryPolicy::with_config(config);

        assert_eq!(
            retry_policy.next_sleep_duration(1),
            Some(Duration::from_secs(100))
        );
        assert_eq!(
            retry_policy.next_sleep_duration(2),
            Some(Duration::from_secs(100))
        );
        assert_eq!(
            retry_policy.next_sleep_duration(3),
            Some(Duration::from_secs(100))
        );
        assert_eq!(retry_policy.next_sleep_duration(4), None);
    }

    #[test]
    fn exponential_retry_policy_delays() {
        let config =
            RetryConfig::ExponentialBackoffRetry(ExponentialBackoffRetry {
                max_num_attempts: 5,
                delay_s: Duration::from_secs(10),
                max_delay_s: Duration::from_secs(50),
            });

        let retry_policy = RetryPolicy::with_config(config);

        assert_eq!(
            retry_policy.next_sleep_duration(1),
            Some(Duration::from_secs(10))
        );
        assert_eq!(
            retry_policy.next_sleep_duration(2),
            Some(Duration::from_secs(20))
        );
        assert_eq!(
            retry_policy.next_sleep_duration(3),
            Some(Duration::from_secs(40))
        );
        assert_eq!(
            retry_policy.next_sleep_duration(4),
            Some(Duration::from_secs(50))
        );
        assert_eq!(
            retry_policy.next_sleep_duration(5),
            Some(Duration::from_secs(50))
        );
        assert_eq!(retry_policy.next_sleep_duration(6), None);
    }
}
