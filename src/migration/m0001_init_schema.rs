use sea_orm::Schema;
use sea_orm_migration::prelude::*;

use crate::entities::{
    code_video, interactive_html, knowledge_explanation, knowledge_video, user, user_checkin,
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let schema = Schema::new(manager.get_database_backend());

        let mut user_table = schema.create_table_from_entity(user::Entity);
        user_table.if_not_exists();
        manager.create_table(user_table).await?;

        let mut user_checkin_table = schema.create_table_from_entity(user_checkin::Entity);
        user_checkin_table.if_not_exists();
        manager.create_table(user_checkin_table).await?;

        let mut knowledge_video_table = schema.create_table_from_entity(knowledge_video::Entity);
        knowledge_video_table.if_not_exists();
        manager.create_table(knowledge_video_table).await?;

        let mut code_video_table = schema.create_table_from_entity(code_video::Entity);
        code_video_table.if_not_exists();
        manager.create_table(code_video_table).await?;

        let mut interactive_html_table = schema.create_table_from_entity(interactive_html::Entity);
        interactive_html_table.if_not_exists();
        manager.create_table(interactive_html_table).await?;

        let mut knowledge_explanation_table =
            schema.create_table_from_entity(knowledge_explanation::Entity);
        knowledge_explanation_table.if_not_exists();
        manager.create_table(knowledge_explanation_table).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for table in [
            "knowledge_explanation",
            "interactive_html",
            "code_video",
            "knowledge_video",
            "user_checkin",
            "user",
        ] {
            manager
                .drop_table(
                    Table::drop()
                        .table(Alias::new(table))
                        .if_exists()
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }
}
