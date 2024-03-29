//! `SeaORM` Entity. Generated by sea-orm-codegen 0.11.3

use std::time::Duration;

use chrono::{DateTime, Utc};
use dto::IntoProto;
use lib::prelude::*;
use sea_orm::entity::prelude::*;
use sea_orm::{DeriveActiveEnum, EnumIter, FromJsonQueryResult};
use serde::{Deserialize, Serialize};

#[derive(
    Clone,
    Serialize,
    Deserialize,
    Debug,
    IntoProto,
    PartialEq,
    DeriveEntityModel,
    Eq,
    FromJsonQueryResult,
)]
#[proto(target = "proto::attempts::Attempt")]
#[sea_orm(table_name = "attempts")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    #[proto(required)]
    pub id: AttemptId,
    #[proto(required)]
    pub run_id: RunId,
    #[proto(skip)]
    pub trigger_id: TriggerId,
    #[sea_orm(primary_key, auto_increment = false)]
    #[proto(skip)]
    pub project_id: ValidShardedId<ProjectId>,
    pub status: AttemptStatus,
    #[proto(required)]
    pub details: AttemptDetails,
    pub attempt_num: u32,
    #[proto(required)]
    pub created_at: DateTime<Utc>,
}

impl PaginatedEntity for Entity {
    fn cursor_column() -> Self::Column {
        Column::Id
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

#[derive(Debug, Serialize, Deserialize, IntoProto, Clone, PartialEq, Eq)]
#[proto(target = "proto::attempts::WebhookAttemptDetails")]
pub struct WebhookAttemptDetails {
    pub response_code: Option<i32>,
    #[into_proto(map = "Duration::as_secs_f64", map_by_ref)]
    pub response_latency_s: Duration,
    pub error_message: Option<String>,
}

impl WebhookAttemptDetails {
    pub fn is_success(&self) -> bool {
        (200..=299).contains(&self.response_code.unwrap_or(500))
    }

    pub fn with_error(err: String) -> Self {
        Self {
            response_code: None,
            response_latency_s: Duration::default(),
            error_message: Some(err),
        }
    }
}

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    IntoProto,
    PartialEq,
    Eq,
    FromJsonQueryResult,
)]
#[proto(target = "proto::attempts::AttemptDetails", oneof = "details")]
pub enum AttemptDetails {
    #[proto(name = "Webhook")]
    WebhookAttemptDetails(WebhookAttemptDetails),
}

#[derive(
    Debug,
    Serialize,
    Deserialize,
    IntoProto,
    Clone,
    PartialEq,
    Eq,
    EnumIter,
    DeriveActiveEnum,
)]
#[sea_orm(rs_type = "String", db_type = "String(None)")]
#[proto(target = "proto::attempts::AttemptStatus")]
pub enum AttemptStatus {
    #[sea_orm(string_value = "Succeeded")]
    Succeeded,
    #[sea_orm(string_value = "Failed")]
    Failed,
}
