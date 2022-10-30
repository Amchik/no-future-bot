use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(Clone, Serialize, Debug, DeriveEntityModel)]
#[sea_orm(table_name = "post_media")]
pub struct Model {
    #[sea_orm(primary_key)]
    /// Internal ID of post media
    pub id: u64,
    #[sea_orm(unique)]
    /// Internal ID of post
    pub post_id: u64,

    /// Media type (photo or video)
    pub media_type: MediaType,
    /// URL to media
    pub media_url: String,
}

#[derive(EnumIter, Debug, Clone, PartialEq, Eq, Serialize, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Text")]
#[serde(rename_all = "camelCase")]
pub enum MediaType {
    #[sea_orm(string_value = "photo")]
    Photo,
    #[sea_orm(string_value = "video")]
    Video,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
