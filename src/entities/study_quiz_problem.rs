use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use super::common::ProblemAnswer;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "study_quiz_problem")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub study_quiz_id: i32,
    pub sort_order: i32,
    #[sea_orm(column_type = "Text")]
    pub content: String,
    pub choice_a: String,
    pub choice_b: String,
    pub choice_c: String,
    pub choice_d: String,
    pub answer: ProblemAnswer,
    #[sea_orm(column_type = "Text")]
    pub explanation: String,
    pub chosen_answer: Option<ProblemAnswer>,
    pub bookmarked: bool,
    pub mistake_hidden: bool,
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
}

impl Related<super::study_quiz::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::StudyQuiz.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
