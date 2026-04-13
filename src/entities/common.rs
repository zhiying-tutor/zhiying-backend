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

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(
    rs_type = "String",
    db_type = "String(StringLen::N(1))",
    enum_name = "problem_answer"
)]
pub enum ProblemAnswer {
    #[sea_orm(string_value = "A")]
    A,
    #[sea_orm(string_value = "B")]
    B,
    #[sea_orm(string_value = "C")]
    C,
    #[sea_orm(string_value = "D")]
    D,
}
