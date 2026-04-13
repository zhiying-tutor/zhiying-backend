use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use super::common::ProblemAnswer;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "study_quiz_problem")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub study_quiz_id: i32,
    pub problem_id: i32,
    pub sort_order: i32,
    pub chosen_answer: Option<ProblemAnswer>,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::study_quiz::Entity",
        from = "Column::StudyQuizId",
        to = "super::study_quiz::Column::Id"
    )]
    StudyQuiz,
    #[sea_orm(
        belongs_to = "super::problem::Entity",
        from = "Column::ProblemId",
        to = "super::problem::Column::Id"
    )]
    Problem,
}

impl Related<super::study_quiz::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::StudyQuiz.def()
    }
}

impl Related<super::problem::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Problem.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
