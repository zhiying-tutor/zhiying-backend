use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(
    rs_type = "String",
    db_type = "String(StringLen::N(16))",
    enum_name = "knowledge_video_status"
)]
pub enum KnowledgeVideoStatus {
    #[sea_orm(string_value = "QUEUING")]
    Queuing,
    #[sea_orm(string_value = "GENERATING")]
    Generating,
    #[sea_orm(string_value = "FINISHED")]
    Finished,
    #[sea_orm(string_value = "FAILED")]
    Failed,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "knowledge_video")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub user_id: i32,
    pub status: KnowledgeVideoStatus,
    #[sea_orm(column_type = "Text")]
    pub prompt: String,
    pub url: Option<String>,
    pub public: bool,
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
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
