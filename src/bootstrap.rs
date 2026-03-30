use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, Schema};

use crate::entities::{user, user_checkin};

pub async fn bootstrap_database(db: &DatabaseConnection) -> Result<(), DbErr> {
    let backend = db.get_database_backend();
    let schema = Schema::new(backend);

    let mut user_table = schema.create_table_from_entity(user::Entity);
    user_table.if_not_exists();
    db.execute(backend.build(&user_table)).await?;

    let mut user_checkin_table = schema.create_table_from_entity(user_checkin::Entity);
    user_checkin_table.if_not_exists();
    db.execute(backend.build(&user_checkin_table)).await?;

    Ok(())
}
