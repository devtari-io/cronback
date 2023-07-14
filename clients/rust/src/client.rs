use cronback_api_model::{Run, RunMode, RunTrigger};
use http::Method;
use reqwest::IntoUrl;
use serde::de::DeserializeOwned;
use serde::Serialize;
use tracing::log::info;
use url::Url;

use crate::constants::{BASE_URL_ENV, DEFAULT_BASE_URL};
use crate::{
    Error,
    Paginated,
    Pagination,
    Response,
    Result,
    Trigger,
    TriggersFilter,
};
/// An asynchronous client for a cronback API service.
///
/// The client has various configuration options, but has reasonable defaults
/// that should suit most use-cases. To configure a client, use
/// [`Client::builder()`] or [`ClientBuilder::new()`]
///
/// a `Client` manages an internal connection pool, it's designed to be created
/// once and reused (via `Client::clone()`). You do **not** need to wrap
/// `Client` in [`Rc`] or [`Arc`] to reuse it.
///
/// [`Rc`]: std::rc::Rc
#[derive(Clone)]
pub struct Client {
    http_client: reqwest::Client,
    config: ClientConfig,
}

/// A `ClientBuilder` is what should be used to construct a `Client` with custom
/// configuration.
///
/// We default to the production cronback service `https://api.cronback.me/` unless `CRONBACK_BASE_URL`
/// enviornment variable is defined. Alternatively, the `base_url` can be used
/// to override the server url for this particular client instance.
#[must_use]
#[derive(Default, Clone)]
pub struct ClientBuilder {
    config: Config,
}

impl ClientBuilder {
    /// Construct a new client builder with reasonable defaults. Use
    /// [`ClientBuilder::build`] to construct a client.
    pub fn new() -> Self {
        Self {
            config: Config::default(),
        }
    }

    pub fn base_url<T: IntoUrl>(mut self, base_url: T) -> Result<Self> {
        let mut base_url = base_url.into_url()?;
        // We want to make sure that the query string is empty.
        base_url.set_query(None);
        self.config.base_url = Some(base_url);
        Ok(self)
    }

    pub fn secret_token(mut self, secret_token: String) -> Self {
        self.config.secret_token = Some(secret_token);
        self
    }

    /// Construct cronback client.
    pub fn build(self) -> Result<Client> {
        let http_client = match self.config.reqwest_client {
            | Some(c) => c,
            | None => {
                reqwest::ClientBuilder::new()
                    .redirect(reqwest::redirect::Policy::none())
                    .build()?
            }
        };

        let base_url = match self.config.base_url {
            | Some(c) => c,
            | None => {
                // Attempt to read from enviornment variable before fallback to
                // default.
                std::env::var(BASE_URL_ENV)
                    .ok()
                    .map(|base_url| Url::parse(&base_url))
                    .unwrap_or(Ok(DEFAULT_BASE_URL.clone()))
                    .expect("Config::default()")
            }
        };
        Ok(Client {
            http_client,
            config: ClientConfig {
                base_url,
                secret_token: self
                    .config
                    .secret_token
                    .ok_or(Error::SecretTokenRequired)?,
            },
        })
    }

    /// Use a pre-configured [`request::Client`] instance instead of creating
    /// our own. This allows customising TLS, timeout, and other low-level http
    /// client configuration options.
    pub fn reqwest_client(mut self, c: reqwest::Client) -> Self {
        self.config.reqwest_client = Some(c);
        self
    }
}

impl Client {
    /// Constructs a new client with the default configuration. This is **not**
    /// the recommended way to construct a client. We recommend using
    /// `Client::builder().build()` instead.
    ///
    /// # Panics
    ///
    /// This method panics if TLS backend cannot be initialised, or the
    /// underlying resolver cannot load the system configuration. Use
    /// [`Client::builder()`] if you wish to handle the failure as an
    /// [`crate::Error`] instead of panicking.
    pub fn new() -> Self {
        Self::builder().build().expect("Client::new()")
    }

    /// Creates a `ClientBuilder` to configure a `Client`.
    ///
    /// This is the same as `ClientBuilder::new()`.
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    /// Retrieve a trigger by name.
    pub async fn get_trigger<T>(&self, name: T) -> Result<Response<Trigger>>
    where
        T: AsRef<str>,
    {
        let path = format!("/v1/triggers/{}", name.as_ref());
        let path = self.config.base_url.join(&path)?;

        self.execute_request(Method::GET, path).await
    }

    /// Retrieve list of triggers for a project.
    pub async fn list_triggers(
        &self,
        pagination: Option<Pagination>,
        filter: Option<TriggersFilter>,
    ) -> Result<Response<Paginated<Trigger>>> {
        let mut path = self.config.base_url.join("/v1/triggers")?;
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

        self.execute_request(Method::GET, path).await
    }

    /// Cancel a `scheduled` trigger.
    pub async fn cancel_trigger<T>(&self, name: T) -> Result<Response<Trigger>>
    where
        T: AsRef<str>,
    {
        let path = format!("/v1/triggers/{}/cancel", name.as_ref());
        let path = self.config.base_url.join(&path)?;

        self.execute_request(Method::POST, path).await
    }

    /// Pause a `scheduled` trigger.
    pub async fn pause_trigger<T>(&self, name: T) -> Result<Response<Trigger>>
    where
        T: AsRef<str>,
    {
        let path = format!("/v1/triggers/{}/pause", name.as_ref());
        let path = self.config.base_url.join(&path)?;

        self.execute_request(Method::POST, path).await
    }

    /// Resume a `paused` trigger.
    pub async fn resume_trigger<T>(&self, name: T) -> Result<Response<Trigger>>
    where
        T: AsRef<str>,
    {
        let path = format!("/v1/triggers/{}/resume", name.as_ref());
        let path = self.config.base_url.join(&path)?;

        self.execute_request(Method::POST, path).await
    }

    /// Run the trigger immediately
    pub async fn run_trigger<T>(
        &self,
        name: T,
        mode: RunMode,
    ) -> Result<Response<Run>>
    where
        T: AsRef<str>,
    {
        let path = format!("/v1/triggers/{}/run", name.as_ref());
        let path = self.config.base_url.join(&path)?;

        let body = RunTrigger { mode };

        self.execute_request_body(Method::POST, path, body).await
    }

    async fn execute_request<T>(
        &self,
        method: http::Method,
        url: Url,
    ) -> Result<Response<T>>
    where
        T: DeserializeOwned,
    {
        info!("Sending a request '{} {}'", method, url);
        let request = self.http_client.request(method, url);
        let resp = request
            .bearer_auth(&self.config.secret_token)
            .send()
            .await?;
        Response::from_raw_response(resp).await
    }

    async fn execute_request_body<T, B>(
        &self,
        method: http::Method,
        url: Url,
        body: B,
    ) -> Result<Response<T>>
    where
        T: DeserializeOwned,
        B: Serialize + std::fmt::Debug,
    {
        info!("Sending a request '{} {}': {:?}", method, url, body);
        let request = self.http_client.request(method, url);
        let resp = request
            .bearer_auth(&self.config.secret_token)
            .json(&body)
            .send()
            .await?;
        Response::from_raw_response(resp).await
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Default, Clone)]
struct Config {
    base_url: Option<Url>,
    secret_token: Option<String>,
    reqwest_client: Option<reqwest::Client>,
}

#[derive(Clone)]
struct ClientConfig {
    base_url: Url,
    secret_token: String,
}

// Ensure that Client is Send + Sync. Compiler will fail if it's not.
const _: () = {
    fn assert_send<T: Send + Sync>() {}
    let _ = assert_send::<Client>;
};

#[cfg(test)]
mod tests {}
