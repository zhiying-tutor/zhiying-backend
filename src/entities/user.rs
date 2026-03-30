use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use crate::entities::common::Gender;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "user")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub username: String,
    pub password: String,
    pub last_login: Option<DateTimeUtc>,
    pub birth_year: Option<i32>,
    pub gender: Option<Gender>,
    pub introduction: String,
    pub exp: i32,
    pub gold: i32,
    pub diamond: i32,
    pub total_checkin: i32,
    pub streak_checkin: i32,
    pub last_checkin: Option<Date>,
    pub invited_by: Option<i32>,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::user_checkin::Entity")]
    UserCheckins,
}

impl Related<super::user_checkin::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserCheckins.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
