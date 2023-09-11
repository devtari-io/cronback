use async_trait::async_trait;
use http::header::{self, USER_AGENT};
use reqwest::{IntoUrl, RequestBuilder};
use serde::de::DeserializeOwned;
use serde::Serialize;
use url::Url;

use crate::constants::{BASE_URL_ENV, DEFAULT_BASE_URL};
use crate::{Error, Response, Result};

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

#[async_trait]
pub trait RequestRunner: Sync + Send {
    fn prepare_request(
        &self,
        method: http::Method,
        path: Url,
    ) -> Result<RequestBuilder>;

    fn make_url(&self, path: &str) -> Result<Url>;

    fn prepare_request_with_body<B>(
        &self,
        method: http::Method,
        path: Url,
        body: B,
    ) -> Result<RequestBuilder>
    where
        B: Serialize + std::fmt::Debug,
    {
        Ok(self.prepare_request(method, path)?.json(&body))
    }

    async fn process_response<T>(
        &self,
        response: reqwest::Response,
    ) -> Result<Response<T>>
    where
        T: DeserializeOwned + Send,
    {
        Response::from_raw_response(response).await
    }

    async fn run<T>(
        &self,
        method: http::Method,
        path: Url,
    ) -> Result<Response<T>>
    where
        T: DeserializeOwned + Send,
    {
        let request = self.prepare_request(method, path)?;
        let resp = request.send().await?;
        self.process_response(resp).await
    }

    async fn run_with_body<T, B>(
        &self,
        method: http::Method,
        path: Url,
        body: B,
    ) -> Result<Response<T>>
    where
        T: DeserializeOwned + Send,
        B: Serialize + std::fmt::Debug + Send,
    {
        let request = self.prepare_request_with_body(method, path, body)?;
        let resp = request.send().await?;
        self.process_response(resp).await
    }
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

    /// If the secret_token is an admin key, the client will act on behalf of
    /// the project passed here.
    /// This method is for cronback admin use only. For normal users, the
    /// project id is infered from the secret token and this value is just
    /// ignored.
    pub fn on_behalf_of(mut self, project_id: String) -> Self {
        self.config.on_behalf_of = Some(project_id);
        self
    }

    /// Construct cronback client.
    pub fn build(self) -> Result<Client> {
        let user_agent = format!(
            "rust-{}-{}-{}",
            env!("CARGO_PKG_VERSION"),
            std::env::consts::OS,
            std::env::consts::ARCH,
        );

        let mut headers = header::HeaderMap::new();
        headers.insert(
            USER_AGENT,
            header::HeaderValue::from_str(&user_agent).expect("User-Agent"),
        );

        if let Some(prj) = &self.config.on_behalf_of {
            headers.insert(
                "X-On-Behalf-Of",
                header::HeaderValue::from_str(prj).expect("X-On-Behalf-Of"),
            );
        }

        let http_client = match self.config.reqwest_client {
            | Some(c) => c,
            | None => {
                reqwest::ClientBuilder::new()
                    .redirect(reqwest::redirect::Policy::none())
                    .default_headers(headers)
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
    on_behalf_of: Option<String>,
    reqwest_client: Option<reqwest::Client>,
}

#[derive(Clone)]
pub(crate) struct ClientConfig {
    pub base_url: Url,
    secret_token: String,
}

// Ensure that Client is Send + Sync. Compiler will fail if it's not.
const _: () = {
    const fn assert_send<T: Send + Sync>() {}
    assert_send::<Client>();
};

#[async_trait]
impl RequestRunner for Client {
    fn make_url(&self, path: &str) -> Result<Url> {
        Ok(self.config.base_url.join(path)?)
    }

    fn prepare_request(
        &self,
        method: http::Method,
        url: Url,
    ) -> Result<RequestBuilder> {
        let request = self
            .http_client
            .request(method, url)
            .bearer_auth(&self.config.secret_token);

        Ok(request)
    }
}

#[cfg(test)]
mod tests {}
