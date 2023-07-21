//! `SeaORM` Entity. Generated by sea-orm-codegen 0.11.3

use chrono::{DateTime, Utc};
use dto::{FromProto, IntoProto};
use lib::model::ValidShardedId;
use lib::types::ProjectId;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "projects")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: ValidShardedId<ProjectId>,
    pub created_at: DateTime<Utc>,
    pub changed_at: DateTime<Utc>,
    pub status: ProjectStatus,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    EnumIter,
    IntoProto,
    FromProto,
    DeriveActiveEnum,
)]
#[sea_orm(rs_type = "String", db_type = "String(None)")]
#[proto(target = "proto::project_svc_proto::ProjectStatus")]
pub enum ProjectStatus {
    #[sea_orm(string_value = "Enabled")]
    Enabled,
    #[sea_orm(string_value = "Disabled")]
    Disabled,
    #[sea_orm(string_value = "QuotaExceeded")]
    QuotaExceeded,
    #[sea_orm(string_value = "PendingDeletion")]
    PendingDeletion,
}

impl ProjectStatus {
    /// An active project is a project that's allowed to schedule new runs
    pub fn is_active(&self) -> bool {
        match self {
            | ProjectStatus::Enabled => true,
            | ProjectStatus::Disabled
            | ProjectStatus::QuotaExceeded
            | ProjectStatus::PendingDeletion => false,
        }
    }
}
