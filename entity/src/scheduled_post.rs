use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(Clone, Serialize, Debug, DeriveEntityModel)]
#[sea_orm(table_name = "schedule_posts")]
pub struct Model {
    #[sea_orm(primary_key)]
    /// Internal ID of post
    pub id: i64,
    /// ID of user, who posted
    pub user_id: i64,

    /// Media internal ids (`post_media`) splitted by ','
    pub media_ids: String,
    /// Post text
    pub post_text: String,
    /// Post source text
    pub post_source: String,
    /// Post source url
    pub post_source_url: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::telegram_user::Entity",
        from = "Column::UserId",
        to = "super::telegram_user::Column::Id"
    )]
    User,
}

impl Related<super::telegram_user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
