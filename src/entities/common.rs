use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(
    rs_type = "String",
    db_type = "String(StringLen::N(16))",
    enum_name = "gender"
)]
pub enum Gender {
    #[sea_orm(string_value = "MALE")]
    Male,
    #[sea_orm(string_value = "FEMALE")]
    Female,
}
