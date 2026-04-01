use sea_orm::Schema;
use sea_orm_migration::prelude::*;

use crate::entities::{user, user_checkin};

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

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(Alias::new("user_checkin"))
                    .if_exists()
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(
                Table::drop()
                    .table(Alias::new("user"))
                    .if_exists()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}
