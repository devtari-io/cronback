use std::time::Duration;

use shared::types::{ExponentialBackoffRetry, RetryConfig, SimpleRetry};

pub struct RetryPolicy {
    config: Option<RetryConfig>,
    num_retries: u32,
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
            num_retries: 0,
            retries_limit,
        }
    }

    pub fn no_retry() -> Self {
        Self {
            num_retries: 0,
            config: None,
            retries_limit: 0,
        }
    }

    /// Returns the duration to sleep if a retry should be done
    /// or None if we should no longer retry
    pub fn next_sleep_duration(&mut self) -> Option<Duration> {
        self.num_retries += 1;
        if self.num_retries > self.retries_limit {
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
            )) => Some(std::cmp::min(
                max_delay_s,
                delay_s * 2_u32.pow(self.num_retries - 1),
            )),
            | None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use shared::types::{ExponentialBackoffRetry, RetryConfig, SimpleRetry};

    use super::RetryPolicy;

    #[test]
    fn no_retry_policy() {
        let mut retry_policy = RetryPolicy::no_retry();

        assert_eq!(retry_policy.next_sleep_duration(), None);
    }

    #[test]
    fn simple_retry_policy() {
        let config = RetryConfig::SimpleRetry(SimpleRetry {
            max_num_attempts: 3,
            delay_s: Duration::from_secs(100),
        });

        let mut retry_policy = RetryPolicy::with_config(config);

        assert_eq!(
            retry_policy.next_sleep_duration(),
            Some(Duration::from_secs(100))
        );
        assert_eq!(
            retry_policy.next_sleep_duration(),
            Some(Duration::from_secs(100))
        );
        assert_eq!(
            retry_policy.next_sleep_duration(),
            Some(Duration::from_secs(100))
        );
        assert_eq!(retry_policy.next_sleep_duration(), None);
    }

    #[test]
    fn exponential_retry_policy() {
        let config =
            RetryConfig::ExponentialBackoffRetry(ExponentialBackoffRetry {
                max_num_attempts: 5,
                delay_s: Duration::from_secs(10),
                max_delay_s: Duration::from_secs(50),
            });

        let mut retry_policy = RetryPolicy::with_config(config);

        assert_eq!(
            retry_policy.next_sleep_duration(),
            Some(Duration::from_secs(10))
        );
        assert_eq!(
            retry_policy.next_sleep_duration(),
            Some(Duration::from_secs(20))
        );
        assert_eq!(
            retry_policy.next_sleep_duration(),
            Some(Duration::from_secs(40))
        );
        assert_eq!(
            retry_policy.next_sleep_duration(),
            Some(Duration::from_secs(50))
        );
        assert_eq!(
            retry_policy.next_sleep_duration(),
            Some(Duration::from_secs(50))
        );
        assert_eq!(retry_policy.next_sleep_duration(), None);
    }
}
