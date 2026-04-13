use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use super::common::ProblemAnswer;

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(
    rs_type = "String",
    db_type = "String(StringLen::N(16))",
    enum_name = "pretest_confidence"
)]
pub enum PretestConfidence {
    #[sea_orm(string_value = "NOT_SURE")]
    NotSure,
    #[sea_orm(string_value = "SOMEWHAT_SURE")]
    SomewhatSure,
    #[sea_orm(string_value = "VERY_SURE")]
    VerySure,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "pretest_problem")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub study_subject_id: i32,
    pub problem_id: i32,
    pub sort_order: i32,
    pub confidence: Option<PretestConfidence>,
    pub chosen_answer: Option<ProblemAnswer>,
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
    #[sea_orm(
        belongs_to = "super::problem::Entity",
        from = "Column::ProblemId",
        to = "super::problem::Column::Id"
    )]
    Problem,
}

impl Related<super::study_subject::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::StudySubject.def()
    }
}

impl Related<super::problem::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Problem.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
