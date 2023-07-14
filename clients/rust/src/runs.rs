use cronback_api_model::{GetRunResponse, Paginated, Pagination, Run};
use http::Method;

use crate::client::RequestRunner;
use crate::{Response, Result};

/// Retrieve list of runs for a given trigger.
pub async fn list<T>(
    client: &impl RequestRunner,
    pagination: Option<Pagination>,
    name: T,
) -> Result<Response<Paginated<Run>>>
where
    T: AsRef<str>,
{
    let path = format!("/v1/triggers/{}/runs", name.as_ref());
    let mut path = client.make_url(&path)?;
    if let Some(pagination) = pagination {
        if let Some(cursor) = pagination.cursor {
            path.query_pairs_mut().append_pair("cursor", &cursor);
        }
        if let Some(limit) = pagination.limit {
            path.query_pairs_mut()
                .append_pair("limit", &limit.to_string());
        }
    }

    client.run(Method::GET, path).await
}

/// Retrieve a run by id.
pub async fn get<T>(
    client: &impl RequestRunner,
    id: T,
) -> Result<Response<GetRunResponse>>
where
    T: AsRef<str>,
{
    let path = format!("/v1/triggers/-/runs/{}", id.as_ref());
    let path = client.make_url(&path)?;

    client.run(Method::GET, path).await
}
