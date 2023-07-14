use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Unexpected error from the http client: {0}")]
    HttpClient(#[from] reqwest::Error),
    #[error("Cannot instantiate cronback client without a secret token!")]
    SecretTokenRequired,
    #[error(transparent)]
    UrlParserError(#[from] url::ParseError),
    #[error("Returned JSON does not conform to protocol: {0}")]
    ProtocolError(#[from] serde_json::Error),
}
