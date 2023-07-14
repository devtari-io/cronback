use std::fmt::Display;

use sea_query::extension::sqlite::SqliteBinOper;
use sea_query::{Alias, ColumnDef, Expr, Iden, Order, Query, SimpleExpr};
use sea_query_binder::SqlxBinder;
use serde::de::DeserializeOwned;
use serde::Serialize;
use sqlx::any::AnyKind;
use sqlx::Row;

use super::errors::DatabaseError;
use super::Database;
use crate::model::ModelId;

#[derive(Iden)]
pub enum KVIden {
    Id,
    Value,
    Project,
    ValueText,
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
        .into_table(table)
        .columns([KVIden::Id, KVIden::Value])
        .values_panic([id.to_string().into(), to_json_value(db, obj)?])
        .build_any_sqlx(db.builder().as_ref());

    sqlx::query_with(&sql, values).execute(&db.pool).await?;
    Ok(())
}

pub async fn update_query<'a, Table, IdType, Type>(
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
    let (sql, values) = Query::update()
        .table(table)
        .values([
            (KVIden::Id, id.to_string().into()),
            (KVIden::Value, to_json_value(db, obj)?),
        ])
        .and_where(Expr::col(KVIden::Id).eq(id.to_string()))
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
        .expr_as(
            Expr::col(KVIden::Value).cast_as(Alias::new("TEXT")),
            KVIden::ValueText,
        )
        .from(table)
        .and_where(Expr::col(KVIden::Id).eq(id.to_string()))
        .build_any_sqlx(db.builder().as_ref());

    let result = sqlx::query_with(&sql, values).fetch_one(&db.pool).await;

    match result {
        | Ok(r) => {
            let j = r.get::<String, _>(KVIden::ValueText.to_string().as_str());
            Ok(Some(serde_json::from_str::<Type>(&j)?))
        }
        | Err(sqlx::Error::RowNotFound) => Ok(None),
        | Err(e) => Err(e.into()),
    }
}

pub async fn paginated_query<'a, Table, IdType, Type>(
    db: &'a Database,
    table: Table,
    filter: SimpleExpr,
    before: &'a Option<IdType>,
    after: &'a Option<IdType>,
    limit: Option<usize>,
) -> Result<Vec<Type>, DatabaseError>
where
    IdType: Display + ModelId,
    Type: DeserializeOwned,
    Table: Iden + 'static,
{
    let (sql, values) = Query::select()
        .column(KVIden::Id)
        .expr_as(
            Expr::col(KVIden::Value).cast_as(Alias::new("TEXT")),
            KVIden::ValueText,
        )
        .from(table)
        .and_where(filter)
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
        .conditions(
            limit.is_some(),
            |q| {
                q.limit(limit.unwrap().try_into().unwrap());
            },
            |_| {},
        )
        .build_any_sqlx(db.builder().as_ref());

    let results = sqlx::query_with(&sql, values)
        .fetch_all(&db.pool)
        .await?
        .into_iter()
        .map(|r| {
            let j = r.get::<String, _>(KVIden::ValueText.to_string().as_str());
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

fn to_json_value(
    db: &Database,
    obj: impl Serialize,
) -> Result<SimpleExpr, DatabaseError> {
    let json_str = serde_json::to_string(&obj)?;

    Ok(match db.pool.any_kind() {
        | AnyKind::Postgres => Expr::val(json_str).cast_as(Alias::new("json")),
        | AnyKind::Sqlite => json_str.into(),
    })
}

// Revisit the need for this if https://github.com/SeaQL/sea-query/issues/632
// gets implemented.
pub trait GeneratedJsonField {
    fn generate_from_json_field(
        &mut self,
        column: impl Iden,
        field: &str,
    ) -> &mut Self;
}

impl GeneratedJsonField for ColumnDef {
    fn generate_from_json_field(
        &mut self,
        column: impl Iden,
        field: &str,
    ) -> &mut Self {
        // The syntax is the same between sqlite and postgres
        self.extra(format!(
            "GENERATED ALWAYS AS ({} ->> '{field}') STORED",
            column.to_string()
        ))
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
