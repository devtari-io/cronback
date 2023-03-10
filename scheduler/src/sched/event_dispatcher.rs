use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};

use proto::{
    dispatcher_proto::DispatchEventRequest,
    event_proto::{self, Event, EventStatus, Request},
    trigger_proto::{self, EventRetryPolicy, OnStatusHandler, Trigger},
};
use shared::{
    grpc_client_provider::DispatcherClientProvider,
    types::{EventId, OwnerId},
};
use tracing::info;

const MAX_ALLOWED_RETRY_DURATION: Duration = Duration::from_secs(60 * 10);

pub(crate) struct DispatchedEvent {
    event: Event,
    on_success: Option<OnStatusHandler>,
    on_failure: Option<OnStatusHandler>,
    retry_policy: Option<EventRetryPolicy>,
    dispatcher_client_provider: Arc<DispatcherClientProvider>,
}

impl DispatchedEvent {
    pub fn from_trigger(
        trigger: Trigger,
        dispatcher_client_provider: Arc<DispatcherClientProvider>,
    ) -> Self {
        DispatchedEvent {
            event: Event {
                id: EventId::new(&OwnerId(trigger.owner_id)).into(),
                trigger_id: trigger.id,
                started_at: Some(SystemTime::now().into()),
                request: Some(Request {
                    emit: trigger.emit,
                    request_payload: trigger.payload,
                }),
                status: EventStatus::New.into(),
            },
            on_success: trigger.on_success,
            on_failure: trigger.on_failure,
            retry_policy: trigger.event_retry_policy,
            dispatcher_client_provider,
        }
    }

    pub fn id(&self) -> &str {
        &self.event.id
    }

    pub async fn run(self) {
        // TODO: How to handle infra failures?
        let mut client = self
            .dispatcher_client_provider
            .get_or_create()
            .await
            .unwrap();

        let mut retry_policy: RetryPolicy = self
            .retry_policy
            .clone()
            .map(|p| p.into())
            .unwrap_or_else(RetryPolicy::no_retry);

        loop {
            let response = client
                .dispatch_event(DispatchEventRequest {
                    event: Some(self.event.clone()),
                })
                .await
                .unwrap() // TODO: How to handle infra failures?
                .into_inner();

            match response.status() {
                | event_proto::EventInstanceStatus::Success => {
                    info!(
                        "Dispatch for event {} succeeded. Webhook response took {}.",
                        self.id(),
                        response.response.as_ref().unwrap().latency.as_ref().unwrap(),
                    );
                    break;
                }
                | _ => match retry_policy.get_sleep_duration() {
                    | None => {
                        info!(
                            "Dispatch for event {} failed with status code \"{:?}\": {}. Exhausted all retries. Giving up.",
                            self.id(),
                            response.status(),
                            response.error_message.as_ref().unwrap_or(&"NO_MESSAGE".to_owned()),
                        );
                        break;
                    }
                    | Some(d) => {
                        info!(
                            "Dispatch for event {} failed with status code \"{:?}\": {}. Will retry after {:?}.",
                            self.id(),
                            response.status(),
                            response.error_message.as_ref().unwrap_or(&"NO_MESSAGE".to_owned()),
                            d,
                        );
                        tokio::time::sleep(d).await;
                    }
                },
            }
        }

        // TODO: Record the result to the database
        // TODO: Handle notifications
    }
}

enum RetryDelay {
    NoRetry,
    Simple {
        delay: Duration,
    },
    Exponential {
        delay: Duration,
        max_delay: Duration,
    },
}
struct RetryPolicy {
    num_retries: u32,
    retries_limit: u32,
    delay: RetryDelay,
}

impl RetryPolicy {
    fn no_retry() -> Self {
        RetryPolicy {
            num_retries: 0,
            retries_limit: 0,
            delay: RetryDelay::NoRetry,
        }
    }

    /// Returns the duration to sleep if a retry should be done
    /// or None if we should no longer retry
    fn get_sleep_duration(&mut self) -> Option<Duration> {
        self.num_retries += 1;
        if self.num_retries > self.retries_limit {
            return None;
        }
        match self.delay {
            | RetryDelay::NoRetry => None,
            | RetryDelay::Simple { delay } => Some(delay),
            | RetryDelay::Exponential { delay, max_delay } => {
                Some(std::cmp::min(
                    max_delay,
                    delay * 2_u32.pow(self.num_retries - 1),
                ))
            }
        }
    }
}

impl From<EventRetryPolicy> for RetryPolicy {
    fn from(value: EventRetryPolicy) -> Self {
        RetryPolicy {
            num_retries: 0,
            retries_limit: value.limit as u32,
            delay: match value.policy() {
                | trigger_proto::RetryPolicy::Unknown => RetryDelay::NoRetry,
                | trigger_proto::RetryPolicy::Simple => RetryDelay::Simple {
                    delay: std::cmp::min(
                        value
                            .delay
                            .map(|d| d.try_into().unwrap())
                            .unwrap_or_default(),
                        MAX_ALLOWED_RETRY_DURATION,
                    ),
                },
                | trigger_proto::RetryPolicy::Exponential => {
                    RetryDelay::Exponential {
                        delay: value
                            .delay
                            .map(|d| d.try_into().unwrap())
                            .unwrap_or_default(),
                        max_delay: value
                            .max_delay
                            .map(|d| d.try_into().unwrap())
                            .unwrap_or(MAX_ALLOWED_RETRY_DURATION),
                    }
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use proto::trigger_proto::{self, EventRetryPolicy};

    use super::RetryPolicy;

    #[test]
    fn no_retry_policy() {
        let event_retry_policy = EventRetryPolicy {
            policy: None,
            limit: 3,
            delay: Some(prost_types::Duration {
                seconds: 100,
                nanos: 0,
            }),
            max_delay: None,
            notifications: vec![],
        };

        let mut retry_policy: RetryPolicy = event_retry_policy.into();

        assert_eq!(retry_policy.get_sleep_duration(), None);
    }

    #[test]
    fn simple_retry_policy() {
        let event_retry_policy = EventRetryPolicy {
            policy: Some(trigger_proto::RetryPolicy::Simple.into()),
            limit: 3,
            delay: Some(prost_types::Duration {
                seconds: 100,
                nanos: 0,
            }),
            max_delay: None,
            notifications: vec![],
        };

        let mut retry_policy: RetryPolicy = event_retry_policy.into();

        assert_eq!(
            retry_policy.get_sleep_duration(),
            Some(Duration::from_secs(100))
        );
        assert_eq!(
            retry_policy.get_sleep_duration(),
            Some(Duration::from_secs(100))
        );
        assert_eq!(
            retry_policy.get_sleep_duration(),
            Some(Duration::from_secs(100))
        );
        assert_eq!(retry_policy.get_sleep_duration(), None);
    }

    #[test]
    fn exponential_retry_policy() {
        let event_retry_policy = EventRetryPolicy {
            policy: Some(trigger_proto::RetryPolicy::Exponential.into()),
            limit: 5,
            delay: Some(prost_types::Duration {
                seconds: 10,
                nanos: 0,
            }),
            max_delay: Some(prost_types::Duration {
                seconds: 50,
                nanos: 0,
            }),
            notifications: vec![],
        };

        let mut retry_policy: RetryPolicy = event_retry_policy.into();

        assert_eq!(
            retry_policy.get_sleep_duration(),
            Some(Duration::from_secs(10))
        );
        assert_eq!(
            retry_policy.get_sleep_duration(),
            Some(Duration::from_secs(20))
        );
        assert_eq!(
            retry_policy.get_sleep_duration(),
            Some(Duration::from_secs(40))
        );
        assert_eq!(
            retry_policy.get_sleep_duration(),
            Some(Duration::from_secs(50))
        );
        assert_eq!(
            retry_policy.get_sleep_duration(),
            Some(Duration::from_secs(50))
        );
        assert_eq!(retry_policy.get_sleep_duration(), None);
    }
}
