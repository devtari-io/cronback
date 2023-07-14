use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Unexpected error from the http client: {0}")]
    HttpClient(#[from] reqwest::Error),
}
