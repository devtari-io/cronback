use std::fmt::Display;

use sqlx::{QueryBuilder, Sqlite};

use crate::types::ValidId;

pub fn paginated_query_builder<'a, IdType>(
    table: &'static str,
    filter_key: &'static str,
    filter_value: &'a str,
    before: &'a Option<IdType>,
    after: &'a Option<IdType>,
    limit: usize,
) -> QueryBuilder<'a, Sqlite>
where
    IdType: Display + ValidId,
{
    let mut q: sqlx::QueryBuilder<Sqlite> = sqlx::QueryBuilder::new(format!(
        "SELECT id, value FROM {table} WHERE JSON_EXTRACT(value, \
         '$.{filter_key}') = ",
    ));

    q.push_bind(filter_value);

    if let Some(before) = before {
        q.push("AND id > ");
        q.push_bind(before.value());
    }

    if let Some(after) = after {
        q.push("AND id < ");
        q.push_bind(after.value());
    }

    q.push("ORDER BY id DESC LIMIT");
    q.push_bind(limit as u32);
    q
}
