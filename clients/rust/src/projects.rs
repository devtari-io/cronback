use cronback_api_model::admin::CreateProjectResponse;
use http::Method;

use crate::client::RequestRunner;
use crate::{Response, Result};

/// Creates a new project
pub async fn create(
    client: &impl RequestRunner,
) -> Result<Response<CreateProjectResponse>> {
    let path = "/v1/admin/projects";
    let path = client.make_url(path)?;

    client.run(Method::POST, path).await
}
