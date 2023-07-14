use async_trait::async_trait;
use cronback::client::RequestRunner;
use cronback::{Client, Response, Result};
use reqwest::RequestBuilder;
use serde::de::DeserializeOwned;
use tracing::debug;
use url::Url;

use crate::args::CommonOptions;

pub struct WrappedClient {
    pub common_options: CommonOptions,
    pub inner: Client,
}

#[async_trait]
impl RequestRunner for WrappedClient {
    fn make_url(&self, path: &str) -> Result<Url> {
        self.inner.make_url(path)
    }

    fn prepare_request(
        &self,
        method: http::Method,
        url: Url,
    ) -> Result<RequestBuilder> {
        let request = self.inner.prepare_request(method, url);
        // 1. Log debug information about the request
        // 2. Inject header identifying that this is the CLI client
        let custom_user_agent = format!(
            "cli-{}-{}-{}",
            env!("CARGO_PKG_VERSION"),
            std::env::consts::OS,
            std::env::consts::ARCH,
        );
        let request = request
            .map(|r| r.header(reqwest::header::USER_AGENT, custom_user_agent));
        debug!(?request);
        request
    }

    async fn process_response<T>(
        &self,
        response: reqwest::Response,
    ) -> Result<Response<T>>
    where
        T: DeserializeOwned + Send,
    {
        debug!(?response);
        let response = self.inner.process_response(response).await?;
        if response.status_code() == http::StatusCode::UNAUTHORIZED {
            eprintln!();
            eprintln!(
                "Secret token appears to be rejected by the server. Are you \
                 sure you are using the correct API secret token?"
            );
        };

        // Handle show-meta and other common options
        self.common_options.show_response_meta(&response);
        Ok(response)
    }
}
