use cronback_api_model::admin::{
    APIKeyMetaData,
    ApiKey,
    CreateAPIKeyResponse,
    CreateAPIkeyRequest,
};
use cronback_api_model::Paginated;
use http::Method;

use crate::client::RequestRunner;
use crate::{Response, Result};

/// Generates a new API key
pub async fn gen<T>(
    client: &impl RequestRunner,
    key_name: T,
    metadata: APIKeyMetaData,
) -> Result<Response<CreateAPIKeyResponse>>
where
    T: AsRef<str>,
{
    let path = "/v1/admin/api_keys";
    let path = client.make_url(path)?;

    let body = CreateAPIkeyRequest {
        key_name: key_name.as_ref().to_owned(),
        metadata,
    };

    client.run_with_body(Method::POST, path, body).await
}

/// Lists all api keys associated with this project
pub async fn list(
    client: &impl RequestRunner,
) -> Result<Response<Paginated<ApiKey>>> {
    let path = "/v1/admin/api_keys";
    let path = client.make_url(path)?;

    client.run(Method::GET, path).await
}

/// Revokes an API key given its id
pub async fn revoke<T>(
    client: &impl RequestRunner,
    key_id: T,
) -> Result<Response<()>>
where
    T: AsRef<str>,
{
    let path = format!("/v1/admin/api_keys/{}", key_id.as_ref());
    let path = client.make_url(&path)?;

    client.run(Method::DELETE, path).await
}
