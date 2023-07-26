//! `SeaORM` Entity. Generated by sea-orm-codegen 0.11.3

use chrono::{DateTime, Utc};
use dto::{FromProto, IntoProto};
use lib::prelude::*;
use proto::events::RunMeta;
use sea_orm::entity::prelude::*;

use super::attempts;

#[derive(
    Clone, Debug, IntoProto, FromProto, PartialEq, DeriveEntityModel, Eq,
)]
#[proto(target = "proto::runs::Run")]
#[sea_orm(table_name = "runs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    #[proto(required)]
    pub id: RunId,
    #[proto(required)]
    pub trigger_id: TriggerId,
    #[sea_orm(primary_key, auto_increment = false)]
    #[proto(required)]
    pub project_id: ValidShardedId<ProjectId>,
    #[proto(required)]
    pub created_at: DateTime<Utc>,
    pub payload: Option<Payload>,
    #[proto(required)]
    pub action: Action,
    pub status: RunStatus,
    #[proto(skip)]
    pub latest_attempt_id: Option<AttemptId>,
    #[from_proto(always_none)]
    #[sea_orm(ignore)]
    pub latest_attempt: Option<attempts::Model>,
}

impl PaginatedEntity for Entity {
    fn cursor_column() -> Self::Column {
        Column::Id
    }
}

impl Model {
    /// Metadata used in events tracking
    pub fn meta(&self) -> RunMeta {
        proto::events::RunMeta {
            trigger_id: Some(self.trigger_id.clone().into()),
            run_id: Some(self.id.clone().into()),
        }
    }
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    LatestAttempt,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            | Self::LatestAttempt => {
                Entity::belongs_to(super::attempts::Entity)
                    .from(Column::LatestAttemptId)
                    .to(super::attempts::Column::Id)
                    .into()
            }
        }
    }
}

impl Related<super::attempts::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::LatestAttempt.def()
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
#[proto(target = "proto::runs::RunStatus")]
pub enum RunStatus {
    #[sea_orm(string_value = "Attempting")]
    Attempting,
    #[sea_orm(string_value = "Succeeded")]
    Succeeded,
    #[sea_orm(string_value = "Failed")]
    Failed,
}