use std::fmt::Display;

use sea_query::extension::sqlite::SqliteBinOper;
use sea_query::{Expr, Iden, Order, Query, SimpleExpr};
use sea_query_binder::SqlxBinder;
use serde::de::DeserializeOwned;
use serde::Serialize;
use sqlx::any::AnyKind;
use sqlx::Row;

use super::errors::DatabaseError;
use super::Database;
use crate::types::ShardedId;

#[derive(Iden)]
pub enum KVIden {
    Id,
    Value,
}

pub async fn insert_query<'a, Table, IdType, Type>(
    db: &'a Database,
    table: Table,
    id: &'a IdType,
    obj: &'a Type,
) -> Result<(), DatabaseError>
where
    IdType: Display,
    Type: Serialize,
    Table: Iden + 'static,
{
    let (sql, values) = Query::insert()
        .replace()
        .into_table(table)
        .columns([KVIden::Id, KVIden::Value])
        .values_panic([
            id.to_string().into(),
            serde_json::to_string(obj)?.into(),
        ])
        .build_any_sqlx(db.builder().as_ref());

    sqlx::query_with(&sql, values).execute(&db.pool).await?;
    Ok(())
}

pub async fn get_by_id_query<'a, Table, IdType, Type>(
    db: &'a Database,
    table: Table,
    id: &'a IdType,
) -> Result<Option<Type>, DatabaseError>
where
    IdType: Display,
    Type: DeserializeOwned,
    Table: Iden + 'static,
{
    let (sql, values) = Query::select()
        .column(KVIden::Value)
        .from(table)
        .and_where(Expr::col(KVIden::Id).eq(id.to_string()))
        .build_any_sqlx(db.builder().as_ref());

    let result = sqlx::query_with(&sql, values).fetch_one(&db.pool).await;

    match result {
        | Ok(r) => {
            let j = r.get::<String, _>(KVIden::Value.to_string().as_str());
            Ok(Some(serde_json::from_str::<Type>(&j)?))
        }
        | Err(sqlx::Error::RowNotFound) => Ok(None),
        | Err(e) => Err(e.into()),
    }
}

pub async fn paginated_query<'a, Table, IdType, Type>(
    db: &'a Database,
    table: Table,
    filter_key: &'static str,
    filter_value: &'a str,
    before: &'a Option<IdType>,
    after: &'a Option<IdType>,
    limit: usize,
) -> Result<Vec<Type>, DatabaseError>
where
    IdType: Display + ShardedId,
    Type: DeserializeOwned,
    Table: Iden + 'static,
{
    let (sql, values) = Query::select()
        .columns([KVIden::Id, KVIden::Value])
        .from(table)
        .and_where(
            Expr::col(KVIden::Value)
                .cast_json_field(filter_key, db.pool.any_kind())
                .eq(filter_value),
        )
        .conditions(
            before.is_some(),
            |q| {
                q.and_where(
                    Expr::col(KVIden::Id).gt(before.as_ref().unwrap().value()),
                );
            },
            |_| {},
        )
        .conditions(
            after.is_some(),
            |q| {
                q.and_where(
                    Expr::col(KVIden::Id).lt(after.as_ref().unwrap().value()),
                );
            },
            |_| {},
        )
        .order_by(KVIden::Id, Order::Desc)
        .limit(limit.try_into().unwrap())
        .build_any_sqlx(db.builder().as_ref());

    let results = sqlx::query_with(&sql, values)
        .fetch_all(&db.pool)
        .await?
        .into_iter()
        .map(|r| {
            let j = r.get::<String, _>(KVIden::Value.to_string().as_str());
            serde_json::from_str::<Type>(&j)
        })
        .collect::<Result<Vec<_>, _>>();
    Ok(results?)
}

pub trait JsonField {
    fn get_json_field<T>(self, right: T, kind: AnyKind) -> SimpleExpr
    where
        T: Into<SimpleExpr>;
    fn cast_json_field<T>(self, right: T, kind: AnyKind) -> SimpleExpr
    where
        T: Into<SimpleExpr>;
}

impl JsonField for Expr {
    fn get_json_field<T>(self, right: T, kind: AnyKind) -> SimpleExpr
    where
        T: Into<SimpleExpr>,
    {
        match kind {
            | AnyKind::Postgres => {
                //TODO: This is a temporary hack until sea-query 0.29 is
                // released.
                SimpleExpr::CustomWithExpr(
                    "$1 -> $2".to_string(),
                    vec![self.into(), right.into()],
                )
            }
            | AnyKind::Sqlite => {
                self.binary(SqliteBinOper::GetJsonField, right)
            }
        }
    }

    fn cast_json_field<T>(self, right: T, kind: AnyKind) -> SimpleExpr
    where
        T: Into<SimpleExpr>,
    {
        match kind {
            | AnyKind::Postgres => {
                //TODO: This is a temporary hack until sea-query 0.29 is
                // released.
                SimpleExpr::CustomWithExpr(
                    "$1 ->> $2".to_string(),
                    vec![self.into(), right.into()],
                )
            }
            | AnyKind::Sqlite => {
                self.binary(SqliteBinOper::CastJsonField, right)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use sea_query::{
        Alias,
        Expr,
        PostgresQueryBuilder,
        Query,
        SqliteQueryBuilder,
    };

    use crate::database::helpers::JsonField;

    #[test]
    fn test_get_json_field() {
        // Postgres
        let (sql, _values) = Query::select()
            .expr(Expr::asterisk())
            .from(Alias::new("test"))
            .and_where(
                Expr::col(Alias::new("col"))
                    .get_json_field("test", sqlx::any::AnyKind::Postgres)
                    .eq("value"),
            )
            .build(PostgresQueryBuilder);
        assert_eq!(sql, r#"SELECT * FROM "test" WHERE "col" -> $1 = $2"#);

        // Sqlite
        let (sql, _values) = Query::select()
            .expr(Expr::asterisk())
            .from(Alias::new("test"))
            .and_where(
                Expr::col(Alias::new("col"))
                    .get_json_field("test", sqlx::any::AnyKind::Sqlite)
                    .eq("value"),
            )
            .build(SqliteQueryBuilder);
        assert_eq!(sql, r#"SELECT * FROM "test" WHERE "col" -> ? = ?"#);
    }

    #[test]
    fn test_cast_json_field() {
        // Postgres
        let (sql, _values) = Query::select()
            .expr(Expr::asterisk())
            .from(Alias::new("test"))
            .and_where(
                Expr::col(Alias::new("col"))
                    .cast_json_field("test", sqlx::any::AnyKind::Postgres)
                    .eq("value"),
            )
            .build(PostgresQueryBuilder);
        assert_eq!(sql, r#"SELECT * FROM "test" WHERE "col" ->> $1 = $2"#);

        // Sqlite
        let (sql, _values) = Query::select()
            .expr(Expr::asterisk())
            .from(Alias::new("test"))
            .and_where(
                Expr::col(Alias::new("col"))
                    .cast_json_field("test", sqlx::any::AnyKind::Sqlite)
                    .eq("value"),
            )
            .build(SqliteQueryBuilder);
        assert_eq!(sql, r#"SELECT * FROM "test" WHERE "col" ->> ? = ?"#);
    }
}
