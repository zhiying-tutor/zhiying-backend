use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(
    rs_type = "String",
    db_type = "String(StringLen::N(24))",
    enum_name = "study_subject_status"
)]
pub enum StudySubjectStatus {
    #[sea_orm(string_value = "PRETEST_QUEUING")]
    PretestQueuing,
    #[sea_orm(string_value = "PRETEST_GENERATING")]
    PretestGenerating,
    #[sea_orm(string_value = "PRETEST_READY")]
    PretestReady,
    #[sea_orm(string_value = "PLAN_QUEUING")]
    PlanQueuing,
    #[sea_orm(string_value = "PLAN_GENERATING")]
    PlanGenerating,
    #[sea_orm(string_value = "STUDYING")]
    Studying,
    #[sea_orm(string_value = "FINISHED")]
    Finished,
    #[sea_orm(string_value = "FAILED")]
    Failed,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "study_subject")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub user_id: i32,
    #[sea_orm(column_type = "Text")]
    pub subject: String,
    pub status: StudySubjectStatus,
    pub total_stages: i32,
    pub finished_stages: i32,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id"
    )]
    User,
    #[sea_orm(has_many = "super::pretest_problem::Entity")]
    PretestProblems,
    #[sea_orm(has_many = "super::study_stage::Entity")]
    StudyStages,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::pretest_problem::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PretestProblems.def()
    }
}

impl Related<super::study_stage::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::StudyStages.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
