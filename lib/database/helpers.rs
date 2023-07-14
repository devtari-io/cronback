use std::fmt::Display;

use serde::de::DeserializeOwned;
use serde::Serialize;
use sqlx::{Pool, Row, Sqlite};

use super::errors::DatabaseError;
use crate::types::ValidId;

pub async fn insert_query<'a, IdType, Type>(
    pool: &'a Pool<Sqlite>,
    table: &'static str,
    id: &'a IdType,
    obj: &'a Type,
) -> Result<(), DatabaseError>
where
    IdType: Display,
    Type: Serialize,
{
    let mut q: sqlx::QueryBuilder<Sqlite> = sqlx::QueryBuilder::new(format!(
        "INSERT OR REPLACE INTO {table} (id,value) ",
    ));
    q.push_values([(id, serde_json::to_string(obj)?)], |mut b, (id, obj)| {
        b.push_bind(id.to_string()).push_bind(obj);
    });

    q.build().execute(pool).await?;
    Ok(())
}

pub async fn get_by_id_query<'a, IdType, Type>(
    pool: &'a Pool<Sqlite>,
    table: &'static str,
    id: &'a IdType,
) -> Result<Option<Type>, DatabaseError>
where
    IdType: Display,
    Type: DeserializeOwned,
{
    let mut q: sqlx::QueryBuilder<Sqlite> = sqlx::QueryBuilder::new(format!(
        "SELECT value FROM {table} attempts WHERE id = ",
    ));
    q.push_bind(id.to_string());

    let result = q.build().fetch_one(pool).await;

    match result {
        | Ok(r) => {
            let j = r.get::<String, _>("value");
            Ok(Some(serde_json::from_str::<Type>(&j)?))
        }
        | Err(sqlx::Error::RowNotFound) => Ok(None),
        | Err(e) => Err(e.into()),
    }
}

pub async fn paginated_query<'a, IdType, Type>(
    pool: &'a Pool<Sqlite>,
    table: &'static str,
    filter_key: &'static str,
    filter_value: &'a str,
    before: &'a Option<IdType>,
    after: &'a Option<IdType>,
    limit: usize,
) -> Result<Vec<Type>, DatabaseError>
where
    IdType: Display + ValidId,
    Type: DeserializeOwned,
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

    let results = q
        .build()
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|r| {
            let j = r.get::<String, _>("value");
            serde_json::from_str::<Type>(&j)
        })
        .collect::<Result<Vec<_>, _>>();
    Ok(results?)
}
