use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(
    rs_type = "String",
    db_type = "String(StringLen::N(16))",
    enum_name = "study_stage_status"
)]
pub enum StudyStageStatus {
    #[sea_orm(string_value = "LOCKED")]
    Locked,
    #[sea_orm(string_value = "STUDYING")]
    Studying,
    #[sea_orm(string_value = "FINISHED")]
    Finished,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "study_stage")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub study_subject_id: i32,
    pub title: String,
    #[sea_orm(column_type = "Text")]
    pub description: String,
    pub sort_order: i32,
    pub status: StudyStageStatus,
    pub total_tasks: i32,
    pub finished_tasks: i32,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::study_subject::Entity",
        from = "Column::StudySubjectId",
        to = "super::study_subject::Column::Id"
    )]
    StudySubject,
    #[sea_orm(has_many = "super::study_task::Entity")]
    StudyTasks,
}

impl Related<super::study_subject::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::StudySubject.def()
    }
}

impl Related<super::study_task::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::StudyTasks.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
