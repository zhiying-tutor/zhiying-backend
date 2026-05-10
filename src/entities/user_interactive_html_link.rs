use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "user_interactive_html_link")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub interactive_html_id: i32,
    pub user_id: i32,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id"
    )]
    User,
    #[sea_orm(
        belongs_to = "super::interactive_html::Entity",
        from = "Column::InteractiveHtmlId",
        to = "super::interactive_html::Column::Id",
        on_delete = "Cascade"
    )]
    InteractiveHtml,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::interactive_html::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::InteractiveHtml.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
