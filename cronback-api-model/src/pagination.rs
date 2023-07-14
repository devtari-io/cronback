use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct Pagination {
    pub cursor: Option<String>,
    pub limit: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Paginated<T> {
    #[serde(flatten)]
    pub meta: PageMeta,
    pub data: Vec<T>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PageMeta {
    pub next_cursor: Option<String>,
    pub has_more: bool,
}
