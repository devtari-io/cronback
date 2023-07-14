use cronback_api_model::{
    Paginated,
    Pagination,
    Run,
    RunMode,
    RunTrigger,
    TriggersFilter,
};
use http::Method;

use crate::client::RequestRunner;
use crate::{Response, Result, Trigger};

/// Create a new trigger from JSON definition.
///
/// We intentionally don't accept [`Trigger`] here since API is designed to
/// be more relaxed than the required fields in Trigger model. If we
/// accepted [`Trigger`] as input, the trigger defaults will be set on
/// the client side and not server side. To want to make it easy to
/// change those defaults on the server side without having to release a
/// new client version.
pub async fn create_from_json(
    client: &impl RequestRunner,
    trigger_req: serde_json::Value,
) -> Result<Response<Trigger>> {
    let path = client.make_url("/v1/triggers")?;
    client.run_with_body(Method::POST, path, trigger_req).await
}
/// Retrieve a trigger by name.
pub async fn get<T>(
    client: &impl RequestRunner,
    name: T,
) -> Result<Response<Trigger>>
where
    T: AsRef<str>,
{
    let path = client.make_url(&format!("/v1/triggers/{}", name.as_ref()))?;
    client.run(Method::GET, path).await
}

/// Retrieve list of triggers for a project.
pub async fn list(
    client: &impl RequestRunner,
    pagination: Option<Pagination>,
    filter: Option<TriggersFilter>,
) -> Result<Response<Paginated<Trigger>>> {
    let mut path = client.make_url("/v1/triggers")?;
    if let Some(pagination) = pagination {
        if let Some(cursor) = pagination.cursor {
            path.query_pairs_mut().append_pair("cursor", &cursor);
        }
        if let Some(limit) = pagination.limit {
            path.query_pairs_mut()
                .append_pair("limit", &limit.to_string());
        }
    }

    if let Some(filter) = filter {
        for status in filter.status {
            path.query_pairs_mut()
                .append_pair("status", &status.to_string());
        }
    }

    client.run(Method::GET, path).await
}

/// Cancel a `scheduled` trigger.
pub async fn cancel<T>(
    client: &impl RequestRunner,
    name: T,
) -> Result<Response<Trigger>>
where
    T: AsRef<str>,
{
    let path = format!("/v1/triggers/{}/cancel", name.as_ref());
    let path = client.make_url(&path)?;

    client.run(Method::POST, path).await
}

/// Pause a `scheduled` trigger.
pub async fn pause<T>(
    client: &impl RequestRunner,
    name: T,
) -> Result<Response<Trigger>>
where
    T: AsRef<str>,
{
    let path = format!("/v1/triggers/{}/pause", name.as_ref());
    let path = client.make_url(&path)?;

    client.run(Method::POST, path).await
}

/// Resume a `paused` trigger.
pub async fn resume<T>(
    client: &impl RequestRunner,
    name: T,
) -> Result<Response<Trigger>>
where
    T: AsRef<str>,
{
    let path = format!("/v1/triggers/{}/resume", name.as_ref());
    let path = client.make_url(&path)?;

    client.run(Method::POST, path).await
}

/// Run the trigger immediately
pub async fn run<T>(
    client: &impl RequestRunner,
    name: T,
    mode: RunMode,
) -> Result<Response<Run>>
where
    T: AsRef<str>,
{
    let path = format!("/v1/triggers/{}/run", name.as_ref());
    let path = client.make_url(&path)?;

    let body = RunTrigger { mode };

    client.run_with_body(Method::POST, path, body).await
}

/// Permanently delete a trigger.
pub async fn delete<T>(
    client: &impl RequestRunner,
    name: T,
) -> Result<Response<()>>
where
    T: AsRef<str>,
{
    let path = format!("/v1/triggers/{}", name.as_ref());
    let path = client.make_url(&path)?;

    client.run(Method::DELETE, path).await
}

