use std::sync::Arc;
use std::time::Instant;

use chrono::Utc;
use chrono_tz::UTC;
use futures::FutureExt;
use lib::database::attempt_log_store::AttemptLogStore;
use lib::model::ValidShardedId;
use lib::types::{
    ActionAttemptLog,
    AttemptDetails,
    AttemptLogId,
    AttemptStatus,
    HttpMethod,
    InvocationId,
    InvocationStatus,
    Payload,
    ProjectId,
    TriggerId,
    Webhook,
    WebhookAttemptDetails,
};
use metrics::counter;
use reqwest::header::HeaderValue;
use reqwest::Method;
use tracing::{debug, error, info};
use validator::Validate;

use crate::retry::RetryPolicy;

fn to_reqwest_http_method(method: &HttpMethod) -> reqwest::Method {
    match method {
        | HttpMethod::Get => Method::GET,
        | HttpMethod::Post => Method::POST,
        | HttpMethod::Put => Method::PUT,
        | HttpMethod::Delete => Method::DELETE,
        | HttpMethod::Head => Method::HEAD,
        | HttpMethod::Patch => Method::PATCH,
    }
}

pub struct WebhookActionJob {
    pub invocation_id: InvocationId,
    pub trigger_id: TriggerId,
    pub project: ValidShardedId<ProjectId>,

    pub webhook: Webhook,
    pub payload: Option<Payload>,

    pub attempt_store: Arc<dyn AttemptLogStore + Send + Sync>,
}

impl WebhookActionJob {
    #[tracing::instrument(skip_all, fields(
            invocation_id = %self.invocation_id,
            trigger_id = %self.trigger_id,
            webhook_url = self.webhook.url
            ))]
    pub async fn run(&self) -> InvocationStatus {
        let retry_policy = if let Some(config) = &self.webhook.retry {
            RetryPolicy::with_config(config.clone())
        } else {
            RetryPolicy::no_retry()
        };

        let res = retry_policy
            .retry(|retry_num| {
                {
                    async move {
                        counter!("dispatcher.attempts_total", 1);
                        info!(
                            "Executing retry #{retry_num} for invocation {}",
                            &self.invocation_id,
                        );

                        let attempt_start_time = Utc::now().with_timezone(&UTC);

                        let attempt_id = AttemptLogId::generate(&self.project);
                        let response = dispatch_webhook(
                            &self.trigger_id,
                            &self.invocation_id,
                            &attempt_id,
                            retry_num,
                            &self.webhook,
                            &self.payload,
                        )
                        .await;

                        let attempt_log = ActionAttemptLog {
                            id: attempt_id.clone().into(),
                            invocation: self.invocation_id.clone(),
                            trigger: self.trigger_id.clone(),
                            project: self.project.clone(),
                            status: if response.is_success() {
                                AttemptStatus::Succeeded
                            } else {
                                AttemptStatus::Failed
                            },
                            details: AttemptDetails::WebhookAttemptDetails(
                                response.clone(),
                            ),
                            created_at: attempt_start_time,
                        };

                        info!(
                        attempt_id = %attempt_log.id,
                        status = ?attempt_log.status,
                        "dispatch-attempt"
                        );

                        if let Err(e) =
                            self.attempt_store.log_attempt(&attempt_log).await
                        {
                            error!(
                                "Failed to log attempt {attempt_id} to \
                                 database: {}",
                                e
                            );
                        }

                        if response.is_success() {
                            Ok(())
                        } else {
                            Err(())
                        }
                    }
                }
                .boxed()
            })
            .await;

        match res {
            | Ok(_) => InvocationStatus::Succeeded,
            | Err(_) => InvocationStatus::Failed,
        }
    }
}

#[tracing::instrument(skip(payload))]
async fn dispatch_webhook(
    trigger_id: &TriggerId,
    invocation_id: &InvocationId,
    attempt_id: &AttemptLogId,
    retry_num: u32,
    webhook: &Webhook,
    payload: &Option<Payload>,
) -> WebhookAttemptDetails {
    let validation_result = webhook.validate();

    if let Err(e) = validation_result {
        debug!(
            trigger_id = %trigger_id,
            "Webhook validation failure for trigger '{}': {}",
            trigger_id.to_string(),
            e.to_string(),
        );
        return WebhookAttemptDetails::with_error(format!(
            "Webhook validation failure: {e}"
        ));
    }

    // It's important to not follow any redirects for security reasons.
    // TODO: Reconsider this by hooking into the redirect hooks and re-running
    // the validations on every redirect attempt.
    let http_client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();

    let http_method = to_reqwest_http_method(&webhook.http_method);
    // Custom Cronback headers
    let mut http_headers = reqwest::header::HeaderMap::new();
    http_headers
        .insert("Cronback-Trigger", trigger_id.to_string().parse().unwrap());
    http_headers.insert(
        "Cronback-Invocation",
        invocation_id.to_string().parse().unwrap(),
    );
    // TODO: Consider removing this.
    http_headers
        .insert("Cronback-Attempt", attempt_id.to_string().parse().unwrap());

    http_headers.insert(
        "Cronback-Delivery-Retry-Counter",
        retry_num.to_string().parse().unwrap(),
    );

    if let Some(payload) = payload {
        let Ok(user_headers)  = reqwest::header::HeaderMap::try_from(&payload.headers) else {
            return WebhookAttemptDetails::with_error("Bad request: Invalid header map".to_string());
        };
        // The user headers take precedence over the cronback headers.
        http_headers.extend(user_headers);

        let Ok(content_type) = HeaderValue::from_str(&payload.content_type) else {
            return WebhookAttemptDetails::with_error("Bad request: Invalid content-type header value".to_string());
        };

        http_headers.insert(reqwest::header::CONTENT_TYPE, content_type);
    }

    let request_start_time = Instant::now();
    let mut request = http_client
        .request(http_method, webhook.url.clone().unwrap())
        .headers(http_headers)
        .timeout(webhook.timeout_s);

    if let Some(payload) = payload {
        request = request.body(payload.body.clone());
    }

    let response = request.send().await;
    let latency = request_start_time.elapsed();

    match response {
        | Ok(resp) => {
            WebhookAttemptDetails {
                response_code: Some(resp.status().as_u16() as i32),
                response_latency_s: latency,
                error_message: None,
            }
        }
        | Err(e) => {
            let message = if e.is_connect() {
                "Connection Failed"
            } else if e.is_timeout() {
                "Request timeout"
            } else {
                "Request failed"
            }
            .to_string();

            debug!("Request for attempt '{}' failed with: {:?}", attempt_id, e);

            WebhookAttemptDetails::with_error(message)
        }
    }
}
