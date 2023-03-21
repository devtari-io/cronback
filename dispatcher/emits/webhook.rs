use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use chrono::Utc;
use chrono_tz::UTC;
use metrics::counter;
use reqwest::header::HeaderValue;
use reqwest::Method;
use shared::types::{
    AttemptDetails,
    AttemptLogId,
    AttemptStatus,
    EmitAttemptLog,
    HttpMethod,
    InvocationId,
    OwnerId,
    Payload,
    TriggerId,
    Webhook,
    WebhookAttemptDetails,
    WebhookDeliveryStatus,
};
use tracing::{debug, error, info};

use crate::attempt_log_store::AttemptLogStore;
use crate::retry::RetryPolicy;

fn to_reqwest_http_method(method: &HttpMethod) -> reqwest::Method {
    match method {
        | HttpMethod::GET => Method::GET,
        | HttpMethod::POST => Method::POST,
        | HttpMethod::PUT => Method::PUT,
        | HttpMethod::DELETE => Method::DELETE,
        | HttpMethod::HEAD => Method::HEAD,
        | HttpMethod::PATCH => Method::PATCH,
    }
}

pub struct WebhookEmitJob {
    pub invocation_id: InvocationId,
    pub trigger_id: TriggerId,
    pub owner_id: OwnerId,

    pub webhook: Webhook,
    pub payload: Payload,

    pub attempt_store: Arc<dyn AttemptLogStore + Send + Sync>,
}

impl WebhookEmitJob {
    #[tracing::instrument(skip_all, fields(
            invocation_id = %self.invocation_id,
            trigger_id = %self.trigger_id,
            webhook_url = self.webhook.url
            ))]
    pub async fn run(&self) -> WebhookDeliveryStatus {
        let mut retry_policy = if let Some(config) = &self.webhook.retry {
            RetryPolicy::with_config(config.clone())
        } else {
            RetryPolicy::no_retry()
        };

        loop {
            counter!("dispatcher.attempts_total", 1);

            let attempt_start_time = Utc::now().with_timezone(&UTC);

            let attempt_id = AttemptLogId::new(&self.owner_id);
            let response =
                dispatch_webhook(&attempt_id, &self.webhook, &self.payload)
                    .await;

            let attempt_log = EmitAttemptLog {
                id: attempt_id.clone(),
                invocation_id: self.invocation_id.clone(),
                trigger_id: self.trigger_id.clone(),
                owner_id: self.owner_id.clone(),
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

            if let Err(e) = self.attempt_store.log_attempt(&attempt_log).await {
                error!("Failed to log attempt {attempt_id} to database: {}", e);
            }

            if response.is_success() {
                return WebhookDeliveryStatus::Succeeded;
            }

            match retry_policy.next_sleep_duration() {
                | Some(dur) => {
                    debug!(
                    attempt_id = %attempt_log.id,
                    "dispatch-attempt: will retry in {dur:?}"
                    );
                    tokio::time::sleep(dur).await;
                }
                | None => {
                    debug!(
                    attempt_id = %attempt_log.id,
                    "dispatch-attempt: no more retries left, marking the invocation as failed"
                    );
                    return WebhookDeliveryStatus::Failed;
                }
            }
        }
    }
}

#[tracing::instrument(skip_all)]
async fn dispatch_webhook(
    attempt_id: &AttemptLogId,
    webhook: &Webhook,
    payload: &Payload,
) -> WebhookAttemptDetails {
    // It's important to not follow any redirects for security reasons.
    // TODO: Reconsider this by hooking into the redirect hooks and re-running
    // the validations on every redirect attempt.
    let http_client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();

    let http_method = to_reqwest_http_method(&webhook.http_method);
    let Ok(mut http_headers)  = reqwest::header::HeaderMap::try_from(&payload.headers) else {
        return WebhookAttemptDetails::with_error("Bad request: Invalid header map".to_string());
    };

    let Ok(content_type) = HeaderValue::from_str(&payload.content_type) else {
        return WebhookAttemptDetails::with_error("Bad request: Invalid content-type header value".to_string());
    };
    http_headers.insert(reqwest::header::CONTENT_TYPE, content_type);

    let request_start_time = Instant::now();
    let response = http_client
        .request(http_method, webhook.url.clone().unwrap())
        .headers(http_headers)
        .body(payload.body.clone())
        .timeout(webhook.timeout_s)
        .send()
        .await;
    let latency = request_start_time.elapsed();

    match response {
        | Ok(resp) => {
            WebhookAttemptDetails {
                response_code: Some(resp.status().as_u16() as i32),
                response_payload: Some(Payload {
                    content_type: resp
                        .headers()
                        .get(reqwest::header::CONTENT_TYPE)
                        .map(|v| v.to_str().unwrap_or("INVALID").to_string())
                        .unwrap_or_default(),
                    headers: resp
                        .headers()
                        .iter()
                        .map(|(h, v)| {
                            (
                                h.to_string(),
                                v.to_str().unwrap_or("INVALID").to_owned(),
                            )
                        })
                        .collect::<HashMap<_, _>>(),
                    // TODO: Don't attempt to read the payload if it's larger
                    // than the max allowed payload size
                    // (based on the Content-length
                    // header) TODO: Reconsider the string type for
                    // the payload. This can be a binary blob and the below
                    // unwrap would fail.
                    body: String::from_utf8(
                        resp.bytes().await.unwrap().to_vec(),
                    )
                    .unwrap(),
                }),
                response_latency_s: latency,
                error_msg: None,
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
