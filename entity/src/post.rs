use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(Clone, Serialize, Debug, DeriveEntityModel)]
#[sea_orm(table_name = "posts")]
pub struct Model {
    #[sea_orm(primary_key)]
    /// Internal ID of post
    pub id: u64,
    #[sea_orm(unique)]
    /// Twitter (snowflake) ID of post
    pub platform_id: u64,
    /// Internal ID of author
    pub author_id: u64,

    /// Post text
    pub text: String,
    /// Post source url
    pub source_url: String,
    /// Post source default text
    pub source_text: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
