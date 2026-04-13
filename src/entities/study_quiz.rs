use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(
    rs_type = "String",
    db_type = "String(StringLen::N(16))",
    enum_name = "study_quiz_status"
)]
pub enum StudyQuizStatus {
    #[sea_orm(string_value = "QUEUING")]
    Queuing,
    #[sea_orm(string_value = "GENERATING")]
    Generating,
    #[sea_orm(string_value = "READY")]
    Ready,
    #[sea_orm(string_value = "SUBMITTED")]
    Submitted,
    #[sea_orm(string_value = "FAILED")]
    Failed,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "study_quiz")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub study_task_id: i32,
    pub status: StudyQuizStatus,
    pub cost: i32,
    pub total_problems: i32,
    pub correct_problems: i32,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::study_task::Entity",
        from = "Column::StudyTaskId",
        to = "super::study_task::Column::Id"
    )]
    StudyTask,
    #[sea_orm(has_many = "super::study_quiz_problem::Entity")]
    StudyQuizProblems,
}

impl Related<super::study_task::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::StudyTask.def()
    }
}

impl Related<super::study_quiz_problem::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::StudyQuizProblems.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
