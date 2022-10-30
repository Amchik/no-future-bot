use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(Clone, Serialize, Debug, DeriveEntityModel)]
#[sea_orm(table_name = "posts")]
pub struct Model {
    #[sea_orm(primary_key)]
    /// Internal ID of post
    pub id: i64,
    #[sea_orm(unique)]
    /// Twitter (snowflake) ID of post
    pub platform_id: i64,
    /// Internal ID of author
    pub author_id: i64,

    /// Post text
    pub text: String,
    /// Post source url
    pub source_url: String,
    /// Post source default text
    pub source_text: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::author::Entity",
        from = "Column::AuthorId",
        to = "super::author::Column::Id"
    )]
    Author,
    #[sea_orm(has_many = "super::post_media::Entity")]
    PostMedia,
}

impl Related<super::author::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Author.def()
    }
}

impl Related<super::post_media::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PostMedia.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
