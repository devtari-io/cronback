use std::sync::Arc;

use url::Url;

use crate::constants::{BASE_URL_ENV, DEFAULT_BASE_URL};
use crate::Result;
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
    inner: Arc<CronbackClientRef>,
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
            inner: Arc::new(CronbackClientRef { base_url }),
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
    reqwest_client: Option<reqwest::Client>,
}

struct CronbackClientRef {
    base_url: Url,
}

#[cfg(test)]
mod tests {}
