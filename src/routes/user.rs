use std::ops::Deref;

use rocket::{delete, get, post, routes, serde::json::Json, Route, State};
use sea_orm::{
    ActiveModelBehavior, ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait,
    QueryFilter, Set,
};
use serde::Deserialize;
use telegrambot::teloxide::prelude::*;

use crate::models::{response::APIResponse, telegramauth::TelegramUser};

pub fn routes() -> Vec<Route> {
    routes![get_self, modify_channel, delete_self, get_following_authors]
}

#[derive(Deserialize)]
struct ChannelData {
    channel_id: String,
}

async fn get_or_create_user(db: &DatabaseConnection, user_id: i64) -> entity::telegram_user::Model {
    let user = entity::telegram_user::Entity::find_by_id(user_id)
        .one(db)
        .await
        .unwrap();

    match user {
        Some(u) => u,
        None => {
            let mut active = entity::telegram_user::ActiveModel::new();
            active.id = Set(user_id);

            active.insert(db).await.unwrap()
        }
    }
}

#[get("/")]
async fn get_self(db: &State<DatabaseConnection>, telegram_user: TelegramUser) -> APIResponse {
    APIResponse::new(get_or_create_user(db.deref(), telegram_user.id).await)
}

#[post("/", data = "<data>")]
async fn modify_channel(
    db: &State<DatabaseConnection>,
    telegram_user: TelegramUser,
    bot: &State<Bot>,
    data: Json<ChannelData>,
) -> APIResponse {
    let user =
        telegrambot::commands::link_channel(telegram_user.id, &data.channel_id, &bot, &db).await;

    match user {
        Ok(r) => APIResponse::new(r),
        Err(e) => APIResponse::error(e as u16, e.to_string()),
    }
}

#[delete("/")]
async fn delete_self(db: &State<DatabaseConnection>, telegram_user: TelegramUser) -> APIResponse {
    let result = entity::telegram_user::Entity::delete_by_id(telegram_user.id)
        .exec(db.deref())
        .await
        .unwrap();

    if result.rows_affected == 0 {
        APIResponse::error(401, "You already deleted or never created account")
    } else {
        APIResponse::NoContent
    }
}

#[get("/following")]
async fn get_following_authors(
    db: &State<DatabaseConnection>,
    telegram_user: TelegramUser,
) -> APIResponse {
    let authors = entity::follow::Entity::find()
        .filter(entity::follow::Column::UserId.eq(telegram_user.id))
        .find_also_related(entity::author::Entity)
        .all(db.deref())
        .await
        .unwrap()
        .into_iter()
        .flat_map(|f| f.1)
        .collect::<Vec<_>>();

    APIResponse::new(authors)
}
