use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(Clone, Serialize, Debug, DeriveEntityModel)]
#[sea_orm(table_name = "follows")]
pub struct Model {
    #[sea_orm(primary_key)]
    /// ID of user
    pub user_id: i64,

    #[sea_orm(primary_key)]
    /// ID of author
    pub author_id: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::telegram_user::Entity",
        from = "Column::UserId",
        to = "super::telegram_user::Column::Id"
    )]
    User,
    #[sea_orm(
        belongs_to = "super::author::Entity",
        from = "Column::AuthorId",
        to = "super::author::Column::Id"
    )]
    Author,
}

impl Related<super::telegram_user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::author::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Author.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
