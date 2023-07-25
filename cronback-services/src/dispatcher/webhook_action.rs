use std::sync::Arc;
use std::time::Instant;

use chrono::Utc;
use lib::prelude::*;
use metrics::counter;
use proto::events::AttemptMeta;
use reqwest::header::HeaderValue;
use reqwest::Method;
use tracing::{debug, error, info, warn};
use validator::Validate;

use super::attempt_store::AttemptLogStore;
use super::db_model::attempts::{
    AttemptDetails,
    AttemptStatus,
    WebhookAttemptDetails,
};
use super::db_model::runs::RunStatus;
use super::db_model::*;
use super::retry::Retry;
use super::run_store::RunStore;

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
    pub run: Run,
    pub run_store: Arc<dyn RunStore + Send + Sync>,
    pub attempt_store: Arc<dyn AttemptLogStore + Send + Sync>,
}

impl WebhookActionJob {
    pub async fn run(mut self) -> Run {
        let Action::Webhook(ref webhook) = self.run.action;
        let retry = if let Some(config) = webhook.retry.clone() {
            Retry::with_config(config)
        } else {
            Retry::no_retry()
        };

        info!(
            run_id = %self.run.id,
            "Executing webhook action",
        );

        for delay in retry {
            if self.run.status == RunStatus::Succeeded {
                // No need for further attempts;
                break;
            }
            // Wait for the delay before retrying
            let attempt_num = delay.attempt_number();
            let attempt_limit = delay.attempts_limit();

            if !delay.first_attempt() {
                info!(
                    run_id = %self.run.id,
                    project_id = %self.run.project_id,
                    trigger_id = %self.run.trigger_id,
                    "Previous attempt has failed. Next attempt {}/{} will run after {}s",
                    attempt_num,
                    attempt_limit,
                    delay.duration().as_secs_f32(),
                );
            }
            delay.await;

            info!(
                run_id = %self.run.id,
                project_id = %self.run.project_id,
                trigger_id = %self.run.trigger_id,
                "Executing attempt {}/{} on this run trigger run",
                attempt_num,
                attempt_limit,
            );
            counter!("dispatcher.attempts_total", 1);

            let attempt_start_time = Utc::now();

            let attempt_id = AttemptId::generate(&self.run.project_id);

            let meta = Some(AttemptMeta {
                trigger_id: Some(self.run.trigger_id.clone().into()),
                run_id: Some(self.run.id.clone().into()),
                attempt_id: Some(attempt_id.clone().into()),
            });

            e!(
                project_id = self.run.project_id.clone(),
                WebhookAttemptCreated {
                    meta: meta.clone(),
                    attempt_num,
                    attempt_limit,
                    webhook: Some(webhook.clone().into()),
                }
            );
            // Actually dispatch the webhook
            let response = dispatch_webhook(
                &self.run.trigger_id,
                &self.run.project_id,
                &self.run.id,
                &attempt_id,
                attempt_num,
                webhook,
                &self.run.payload,
            )
            .await;

            // Record the attempt
            let attempt = Attempt {
                id: attempt_id.clone().into(),
                run_id: self.run.id.clone(),
                trigger_id: self.run.trigger_id.clone(),
                project_id: self.run.project_id.clone(),
                status: if response.is_success() {
                    AttemptStatus::Succeeded
                } else {
                    AttemptStatus::Failed
                },
                details: AttemptDetails::WebhookAttemptDetails(
                    response.clone(),
                ),
                attempt_num,
                created_at: attempt_start_time,
            };

            if let Err(e) =
                self.attempt_store.log_attempt(attempt.clone()).await
            {
                error!("Failed to log attempt {attempt_id} to database: {}", e);
            }

            // Record the latest attempt
            self.run.latest_attempt_id = Some(attempt.id);
            // We record the status if successful to avoid an extra DB write
            if response.is_success() {
                self.run.status = RunStatus::Succeeded;
                e!(
                    project_id = self.run.project_id.clone(),
                    WebhookAttemptSucceeded {
                        meta: meta.clone(),
                        attempt_num,
                        attempt_limit,
                        webhook: Some(webhook.clone().into()),
                        response_details: Some(response.clone().into()),
                    }
                );
            } else {
                e!(
                    project_id = self.run.project_id.clone(),
                    WebhookAttemptFailed {
                        meta,
                        attempt_num,
                        attempt_limit,
                        webhook: Some(webhook.clone().into()),
                        response_details: Some(response.clone().into()),
                    }
                );
            }

            if let Err(e) = self.run_store.update_run(self.run.clone()).await {
                // What will happen in case? We will not retry the webhook, but
                // run will be stuck in "attempting" forever!
                // A potential recovery mechanism is to look at the Attempts
                // table (if the attempt was persisted successfully and fix up
                // the run status.
                error!(
                    "Failed to persist run status for run {} for action : {}",
                    self.run.id, e
                );
            }
        }
        // Exhausted all retries, or we succeeded.
        if self.run.status != RunStatus::Succeeded {
            self.run.status = RunStatus::Failed;
            if let Err(e) = self.run_store.update_run(self.run.clone()).await {
                error!(
                    "Failed to persist run status for run {} for action : {}",
                    self.run.id, e
                );
            }
        }
        self.run
    }
}

#[tracing::instrument(skip(payload))]
async fn dispatch_webhook(
    trigger_id: &TriggerId,
    project_id: &ValidShardedId<ProjectId>,
    run_id: &RunId,
    attempt_id: &AttemptId,
    attempt_num: u32,
    webhook: &Webhook,
    payload: &Option<Payload>,
) -> WebhookAttemptDetails {
    let validation_result = webhook.validate();

    if let Err(e) = validation_result {
        // We warn because API validation should have caught this!
        warn!(
            project_id = %project_id,
            trigger_id = %trigger_id,
            run_id = %run_id,
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
    http_headers.insert(RUN_ID_HEADER, run_id.to_string().parse().unwrap());

    http_headers
        .insert(PROJECT_ID_HEADER, project_id.to_string().parse().unwrap());

    http_headers.insert(
        DELIVERY_ATTEMPT_NUM_HEADER,
        attempt_num.to_string().parse().unwrap(),
    );

    if let Some(payload) = payload {
        let Ok(user_headers) =
            reqwest::header::HeaderMap::try_from(&payload.headers)
        else {
            return WebhookAttemptDetails::with_error(
                "Bad request: Invalid header map".to_string(),
            );
        };
        // The user headers take precedence over the cronback headers.
        http_headers.extend(user_headers);

        let Ok(content_type) = HeaderValue::from_str(&payload.content_type)
        else {
            return WebhookAttemptDetails::with_error(
                "Bad request: Invalid content-type header value".to_string(),
            );
        };

        http_headers.insert(reqwest::header::CONTENT_TYPE, content_type);
    }

    let request_start_time = Instant::now();
    let mut request = http_client
        .request(http_method, webhook.url.clone())
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
