#![allow(unused)]

use std::pin::Pin;
use std::time::Duration;

use lib::types::{ExponentialBackoffRetry, RetryConfig, SimpleRetry};

// Inclusive of first attempt? Yes. the initial delay will be zero.
#[derive(Debug)]
pub struct Retry {
    config: Option<RetryConfig>,
    /// Inclusive of the initial attempt. A retry instance that will perform
    /// (no) retries should set this to 1. Zero means that this iterator will
    /// yield no items.
    total_attempts_limit: u32,
    current_attempt: u32,
}

impl Retry {
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
            total_attempts_limit: retries_limit,
            current_attempt: 0,
        }
    }

    pub fn no_retry() -> Self {
        Self {
            config: None,
            total_attempts_limit: 1,
            current_attempt: 0,
        }
    }

    /// Returns the duration to sleep if a retry should be done
    /// or None if we should no longer retry
    fn next_duration(&mut self) -> Option<Duration> {
        if self.current_attempt > self.total_attempts_limit {
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
                    // We subtract 2 because first attempt is not considered a
                    // retry.
                    delay_s * 2_u32.pow(self.current_attempt - 2),
                ))
            }
            | None => None,
        }
    }
}

#[derive(Debug)]
pub struct Delay {
    duration: Duration,
    attempt_counter: u32,
    remaining_attempts: u32,
    sleep: Pin<Box<tokio::time::Sleep>>,
}

impl Delay {
    fn new(
        duration: Duration,
        attempt_counter: u32,
        remaining_attempts: u32,
    ) -> Self {
        Self {
            duration,
            attempt_counter,
            remaining_attempts,
            sleep: Box::pin(tokio::time::sleep(duration)),
        }
    }

    pub fn duration(&self) -> Duration {
        self.duration
    }

    pub fn remaining(&self) -> u32 {
        self.remaining_attempts
    }

    pub fn attempt_number(&self) -> u32 {
        self.attempt_counter
    }

    pub fn attempts_limit(&self) -> u32 {
        self.attempt_counter + self.remaining_attempts
    }

    pub fn first_attempt(&self) -> bool {
        self.attempt_counter == 1
    }

    pub fn last_attempt(&self) -> bool {
        self.remaining_attempts == 0
    }
}

impl std::future::Future for Delay {
    type Output = ();

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        if self.attempt_counter == 1 {
            // This is the retry 0 which mean that we don't sleep or yield the
            // scheduler. This future is immediately resolved to
            // ready.
            return std::task::Poll::Ready(());
        }
        // sleep until the duration is over.
        self.sleep.as_mut().poll(cx)
    }
}

impl Iterator for Retry {
    type Item = Delay;

    fn next(&mut self) -> Option<Self::Item> {
        self.current_attempt += 1;
        if self.current_attempt == 1 {
            // first attempt, always return duration zero.
            return Some(Delay::new(
                Duration::ZERO,
                self.current_attempt,
                // clamp to zero.
                std::cmp::max(
                    0,
                    self.total_attempts_limit - self.current_attempt,
                ),
            ));
        }

        match self.next_duration() {
            | Some(d) => {
                let jitter = Duration::from_millis(
                    (rand::random::<u16>() % 1000).into(),
                );
                Some(Delay::new(
                    d + jitter,
                    self.current_attempt,
                    // clamp to zero.
                    std::cmp::max(
                        0,
                        self.total_attempts_limit - self.current_attempt,
                    ),
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

    use super::Retry;

    #[tokio::test]
    async fn no_retry_policy() {
        let mut retry_policy = Retry::no_retry();
        assert_eq!(1, retry_policy.total_attempts_limit);

        let delay = retry_policy.next().unwrap();
        assert_eq!(1, delay.attempt_number());
        assert_eq!(0, delay.remaining());
        assert_eq!(0, delay.duration().as_millis());
        assert!(delay.first_attempt());
        assert!(delay.last_attempt());

        assert!(retry_policy.next().is_none());
    }

    #[tokio::test]
    async fn simple_retry_policy_delays() {
        let dur = Duration::from_secs(100);
        let config = RetryConfig::SimpleRetry(SimpleRetry {
            max_num_attempts: 3,
            delay_s: dur,
        });

        let mut retry_policy = Retry::with_config(config);

        {
            // Attempt 1
            let delay = retry_policy.next().unwrap();
            assert_eq!(3, delay.attempts_limit());
            assert_eq!(1, delay.attempt_number());
            assert_eq!(2, delay.remaining());
            assert_eq!(0, delay.duration().as_millis());
            assert!(delay.first_attempt());
            assert!(!delay.last_attempt());
        }

        {
            // Attempt 2
            let delay = retry_policy.next().unwrap();
            assert_eq!(2, delay.attempt_number());
            assert_eq!(1, delay.remaining());
            assert!(delay.duration().as_millis() - dur.as_millis() <= 1000);
            assert!(!delay.first_attempt());
            assert!(!delay.last_attempt());
        }

        {
            // Attempt 3
            let delay = retry_policy.next().unwrap();
            assert_eq!(3, delay.attempt_number());
            assert_eq!(0, delay.remaining());
            assert!(delay.duration().as_millis() - dur.as_millis() <= 1000);
            assert!(!delay.first_attempt());
            assert!(delay.last_attempt());
        }
        // No more attempts
        assert!(retry_policy.next().is_none());
    }

    #[tokio::test]
    async fn exponential_retry_policy_delays() {
        let min_dur = Duration::from_secs(10);
        let max_dur = Duration::from_secs(50);
        let config =
            RetryConfig::ExponentialBackoffRetry(ExponentialBackoffRetry {
                max_num_attempts: 5,
                delay_s: min_dur,
                max_delay_s: max_dur,
            });

        let mut retry_policy = Retry::with_config(config);

        {
            // Attempt 1
            let delay = retry_policy.next().unwrap();
            assert_eq!(5, delay.attempts_limit());
            assert_eq!(1, delay.attempt_number());
            assert_eq!(4, delay.remaining());
            assert_eq!(0, delay.duration().as_millis());
            assert!(delay.first_attempt());
            assert!(!delay.last_attempt());
        }

        {
            // Attempt 2
            let delay = retry_policy.next().unwrap();
            assert_eq!(2, delay.attempt_number());
            assert_eq!(3, delay.remaining());
            // minimum duration in attempt 2.
            //
            //
            // CONTINUE FIXING TESTS (subtract durations like below)
            // Then open PR
            // Next is to look at cleaning up the latest_attempts field, do we
            // still need it?
            dbg!(delay.duration());
            assert!(delay.duration().as_millis() - min_dur.as_millis() <= 1000);
            assert!(!delay.first_attempt());
            assert!(!delay.last_attempt());
        }

        {
            // Attempt 3
            let delay = retry_policy.next().unwrap();
            assert_eq!(3, delay.attempt_number());
            assert_eq!(2, delay.remaining());
            // minimum duration * (2 ^ 1) in attempt 3.
            assert!(
                delay.duration().as_millis()
                    - Duration::from_secs(20).as_millis()
                    <= 1000
            );
            assert!(
                delay.duration().as_millis()
                    - Duration::from_secs(20).as_millis()
                    <= 1000
            );
            assert!(!delay.first_attempt());
            assert!(!delay.last_attempt());
        }

        {
            // Attempt 4
            let delay = retry_policy.next().unwrap();
            assert_eq!(4, delay.attempt_number());
            assert_eq!(1, delay.remaining());
            // minimum duration * (2 ^ 2)
            assert!(
                delay.duration().as_millis()
                    - Duration::from_secs(40).as_millis()
                    <= 1000
            );
            assert!(!delay.first_attempt());
            assert!(!delay.last_attempt());
        }

        {
            // Attempt 5
            let delay = retry_policy.next().unwrap();
            assert_eq!(5, delay.attempt_number());
            assert_eq!(0, delay.remaining());
            // minimum duration * (2 ^ 3) but clamped to 50 (max_dur)
            assert!(delay.duration().as_millis() - max_dur.as_millis() <= 1000);
            assert!(!delay.first_attempt());
            assert!(delay.last_attempt());
        }

        // No more attempts
        assert!(retry_policy.next().is_none());
    }
}
