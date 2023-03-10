use std::{
    collections::HashMap,
    time::{Duration, SystemTime},
};

use anyhow::{anyhow, Result};

use proto::{
    dispatcher_proto::DispatchEventResponse,
    event_proto::Response,
    trigger_proto::{HttpMethod, Payload, Webhook},
};
use reqwest::Method;

use proto::event_proto::EventInstanceStatus;

fn to_reqwest_http_method(method_int: i32) -> Result<reqwest::Method> {
    let method =
        HttpMethod::from_i32(method_int).unwrap_or(HttpMethod::Unknown);
    match method {
        | HttpMethod::Unknown => {
            Err(anyhow!("Invalid http method enum value: {}", method_int))
        }
        | HttpMethod::Get => Ok(Method::GET),
        | HttpMethod::Post => Ok(Method::POST),
        | HttpMethod::Put => Ok(Method::PUT),
        | HttpMethod::Delete => Ok(Method::DELETE),
        | HttpMethod::Head => Ok(Method::HEAD),
        | HttpMethod::Patch => Ok(Method::PATCH),
    }
}

pub(crate) async fn dispatch_webhook(
    webhook: &Webhook,
    payload: &Payload,
) -> DispatchEventResponse {
    let response = dispatch_webhook_impl(webhook, payload).await;

    response.unwrap_or_else(|e| DispatchEventResponse {
        status: EventInstanceStatus::InvalidRequest.into(),
        response: None,
        error_message: Some(format!("Invalid request: {e}")),
    })
}

async fn dispatch_webhook_impl(
    webhook: &Webhook,
    payload: &Payload,
) -> Result<DispatchEventResponse> {
    // It's important to not follow any redirects for security reasons.
    // TODO: Reconsider this by hooking into the redirect hooks and re-running
    // the validations on every redirect attempt.
    let http_client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();

    let http_method = to_reqwest_http_method(webhook.http_method)?;
    let mut http_headers: reqwest::header::HeaderMap = (&payload.headers)
        .try_into()
        .map_err(|e| anyhow!("Invalid headers: {}", e))?;
    http_headers.insert(
        reqwest::header::CONTENT_TYPE,
        payload
            .content_type
            .parse()
            .map_err(|e| anyhow!("Invalid content-type header: {}", e))?,
    );
    let http_timeout = Duration::from_secs_f64(webhook.timeout_s);

    let request_start_time = SystemTime::now();
    let response = http_client
        .request(http_method, webhook.url.clone())
        .headers(http_headers)
        .body(payload.body.clone())
        .timeout(http_timeout)
        .send()
        .await;
    let latency = request_start_time.elapsed().unwrap_or_default();
    let latency =
        TryInto::<prost_types::Duration>::try_into(latency).unwrap_or_default();

    Ok(match response {
        | Ok(resp) => DispatchEventResponse {
            status: EventInstanceStatus::Success.into(),
            response: Some(Response {
                http_code: resp.status().as_u16() as i32,
                payload: Some(Payload {
                    content_type: resp
                        .headers()
                        .get(reqwest::header::CONTENT_TYPE)
                        .map(|v| v.to_str().unwrap().to_string())
                        .unwrap_or_default(),
                    headers: resp
                        .headers()
                        .iter()
                        .map(|(h, v)| {
                            (h.to_string(), v.to_str().unwrap().to_owned())
                        })
                        .collect::<HashMap<_, _>>(),
                    // TODO: Don't attempt to read the payload if it's larger than the max allowed payload size (based on the Content-length header)
                    body: resp.bytes().await.unwrap().to_vec(),
                }),
                latency: Some(latency),
            }),
            error_message: None,
        },
        | Err(e) => {
            let status = if e.is_connect() {
                EventInstanceStatus::Connfailed
            } else if e.is_timeout() {
                EventInstanceStatus::Timeout
            } else {
                EventInstanceStatus::Failed
            };

            let response = e.status().map(|status| Response {
                http_code: status.as_u16() as i32,
                payload: None,
                latency: Some(latency),
            });

            DispatchEventResponse {
                status: status.into(),
                response,
                error_message: Some(e.to_string()),
            }
        }
    })
}
