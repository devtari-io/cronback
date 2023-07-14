//! `SeaORM` Entity. Generated by sea-orm-codegen 0.11.3

use chrono::{DateTime, Utc};
use dto::{FromProto, IntoProto};
use sea_orm::entity::prelude::*;

use super::triggers::{Action, Payload};
use crate::database::pagination::PaginatedEntity;
use crate::model::ValidShardedId;
use crate::types::{ProjectId, RunId, TriggerId};

#[derive(Copy, Clone, Default, Debug, DeriveEntity)]
pub struct Entity;

impl EntityName for Entity {
    fn table_name(&self) -> &str {
        "runs"
    }
}

#[derive(
    Clone,
    Debug,
    IntoProto,
    FromProto,
    PartialEq,
    DeriveModel,
    DeriveActiveModel,
    Eq,
)]
#[proto(target = "proto::run_proto::Run")]
pub struct Model {
    #[proto(required)]
    pub id: RunId,
    #[proto(required)]
    pub trigger_id: TriggerId,
    #[proto(required)]
    pub project_id: ValidShardedId<ProjectId>,
    #[proto(required)]
    pub created_at: DateTime<Utc>,
    pub payload: Option<Payload>,
    #[proto(required)]
    pub action: Action,
    pub status: RunStatus,
}

impl PaginatedEntity for Entity {
    fn cursor_column() -> Self::Column {
        Column::Id
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
pub enum Column {
    Id,
    TriggerId,
    ProjectId,
    CreatedAt,
    Payload,
    Action,
    Status,
}

#[derive(Copy, Clone, Debug, EnumIter, DerivePrimaryKey)]
pub enum PrimaryKey {
    Id,
    ProjectId,
}

impl PrimaryKeyTrait for PrimaryKey {
    type ValueType = (String, String);

    fn auto_increment() -> bool {
        false
    }
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {}

impl ColumnTrait for Column {
    type EntityName = Entity;

    fn def(&self) -> ColumnDef {
        match self {
            | Self::Id => ColumnType::String(None).def(),
            | Self::TriggerId => ColumnType::String(None).def(),
            | Self::ProjectId => ColumnType::String(None).def(),
            | Self::CreatedAt => ColumnType::String(None).def(),
            | Self::Payload => ColumnType::String(None).def().null(),
            | Self::Action => ColumnType::String(None).def(),
            | Self::Status => ColumnType::String(None).def(),
        }
    }
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        panic!("No RelationDef")
    }
}

impl ActiveModelBehavior for ActiveModel {}

#[derive(
    Debug,
    Clone,
    FromProto,
    IntoProto,
    PartialEq,
    Eq,
    EnumIter,
    DeriveActiveEnum,
)]
#[sea_orm(rs_type = "String", db_type = "String(None)")]
#[proto(target = "proto::run_proto::RunStatus")]
pub enum RunStatus {
    #[sea_orm(string_value = "Attempting")]
    Attempting,
    #[sea_orm(string_value = "Succeeded")]
    Succeeded,
    #[sea_orm(string_value = "Failed")]
    Failed,
}
