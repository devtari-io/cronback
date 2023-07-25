use axum::response::IntoResponse;
use axum::Json;
use dto::{FromProto, IntoProto};
use proto::common::PaginationOut;
use serde::{Deserialize, Serialize};
use validator::Validate;

// The query parameters for pagination and cursor management
#[derive(Debug, Deserialize, IntoProto, Validate)]
#[proto(target = "proto::common::PaginationIn")]
pub(crate) struct Pagination {
    pub cursor: Option<String>,
    #[validate(range(
        min = 1,
        max = 100,
        message = "must be between 1 and 100"
    ))]
    #[serde(default = "default_limit")]
    pub limit: i32,
}

fn default_limit() -> i32 {
    20
}

// An API model that handles formatting paginated responses into Json
#[derive(Debug, Serialize)]
pub(crate) struct Paginated<T> {
    #[serde(flatten)]
    pub pagination: PageMeta,
    pub data: Vec<T>,
}

impl<T> Paginated<T> {
    pub fn from<B>(data: Vec<B>, pagination: PaginationOut) -> Self
    where
        B: Into<T>,
    {
        Self {
            data: data.into_iter().map(Into::into).collect(),
            pagination: pagination.into(),
        }
    }
}

#[derive(Debug, Serialize, FromProto)]
#[proto(target = "proto::common::PaginationOut")]
pub(crate) struct PageMeta {
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

impl<T> IntoResponse for Paginated<T>
where
    T: Serialize,
{
    fn into_response(self) -> axum::response::Response {
        (axum::http::StatusCode::OK, Json(self)).into_response()
    }
}
