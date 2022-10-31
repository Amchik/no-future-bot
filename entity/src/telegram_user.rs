use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(Clone, Serialize, Debug, DeriveEntityModel)]
#[sea_orm(table_name = "telegram_user")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    /// Telegram ID of user
    pub id: i64,
    #[sea_orm(nullable)]
    /// Telegram channel to post
    pub channel: Option<i64>,
    #[sea_orm(default_value = 0)]
    /// Power level
    pub power_level: i32,
    #[sea_orm(default_value = 0)]
    #[serde(skip)]
    /// Last post ID, that user read in feed
    pub last_feed_id: i64,
}

pub const POWER_USER: i32 = 0;
pub const POWER_MOD: i32 = 50;
pub const POWER_ADMIN: i32 = 100;

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::scheduled_post::Entity")]
    Posts,
    #[sea_orm(has_many = "super::follow::Entity")]
    Follows,
}

impl Related<super::scheduled_post::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Posts.def()
    }
}

impl Related<super::follow::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Follows.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
