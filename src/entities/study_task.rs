use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(
    rs_type = "String",
    db_type = "String(StringLen::N(16))",
    enum_name = "study_task_status"
)]
pub enum StudyTaskStatus {
    #[sea_orm(string_value = "LOCKED")]
    Locked,
    #[sea_orm(string_value = "STUDYING")]
    Studying,
    #[sea_orm(string_value = "FINISHED")]
    Finished,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "study_task")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub study_stage_id: i32,
    pub title: String,
    #[sea_orm(column_type = "Text")]
    pub description: String,
    pub sort_order: i32,
    pub status: StudyTaskStatus,
    pub knowledge_video_id: Option<i32>,
    pub interactive_html_id: Option<i32>,
    pub knowledge_explanation_id: Option<i32>,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::study_stage::Entity",
        from = "Column::StudyStageId",
        to = "super::study_stage::Column::Id"
    )]
    StudyStage,
    #[sea_orm(
        belongs_to = "super::knowledge_video::Entity",
        from = "Column::KnowledgeVideoId",
        to = "super::knowledge_video::Column::Id"
    )]
    KnowledgeVideo,
    #[sea_orm(
        belongs_to = "super::interactive_html::Entity",
        from = "Column::InteractiveHtmlId",
        to = "super::interactive_html::Column::Id"
    )]
    InteractiveHtml,
    #[sea_orm(
        belongs_to = "super::knowledge_explanation::Entity",
        from = "Column::KnowledgeExplanationId",
        to = "super::knowledge_explanation::Column::Id"
    )]
    KnowledgeExplanation,
    #[sea_orm(has_many = "super::study_quiz::Entity")]
    StudyQuizzes,
}

impl Related<super::study_stage::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::StudyStage.def()
    }
}

impl Related<super::knowledge_video::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::KnowledgeVideo.def()
    }
}

impl Related<super::interactive_html::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::InteractiveHtml.def()
    }
}

impl Related<super::knowledge_explanation::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::KnowledgeExplanation.def()
    }
}

impl Related<super::study_quiz::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::StudyQuizzes.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
