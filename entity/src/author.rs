use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(Clone, Serialize, Debug, DeriveEntityModel)]
#[sea_orm(table_name = "authors")]
pub struct Model {
    #[sea_orm(primary_key)]
    /// Internal ID of author
    pub id: i64,
    #[sea_orm(unique)]
    /// Twitter (snowflake) ID of author
    pub platform_id: i64,

    /// Name of twitter account (not username)
    pub name: String,
    /// Username of twitter account
    pub username: String,
    #[sea_orm(nullable)]
    /// Avatar of twitter account
    pub avatar_url: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
