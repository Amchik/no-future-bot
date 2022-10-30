use std::ops::Deref;

use rocket::{delete, get, post, routes, serde::json::Json, Route, State};
use sea_orm::{
    ActiveModelBehavior, ActiveModelTrait, DatabaseConnection, EntityTrait, IntoActiveModel, Set,
};
use serde::Deserialize;
use telegrambot::teloxide::prelude::*;

use crate::models::{response::APIResponse, telegramauth::TelegramUser};

pub fn routes() -> Vec<Route> {
    routes![get_self, modify_channel, delete_self]
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
    let chat_id = match data.channel_id.parse::<i64>() {
        Ok(c) => c,
        _ if data.channel_id.starts_with('@') => {
            let chat_id = data.channel_id.deref();

            match bot.get_chat(chat_id.to_string()).await {
                Ok(chat) => chat.id.0,
                _ => return APIResponse::error(404, format!("Chat '{chat_id}' not found")),
            }
        }
        _ => return APIResponse::error(422, "Invalid channel_id format"),
    };

    let members = match bot.get_chat_administrators(ChatId(chat_id)).await {
        Ok(m) => m,
        _ => return APIResponse::error(401, "Bot is not chat administrator"),
    };
    if !members
        .iter()
        .any(|f| f.user.id.0 == telegram_user.id as u64 && f.can_post_messages())
    {
        return APIResponse::error(
            403,
            "You need to be a chat owner or channel administrator to do that",
        );
    }

    // assume bot can post messages...

    let mut user = get_or_create_user(db.deref(), telegram_user.id)
        .await
        .into_active_model();
    user.channel = Set(Some(chat_id));

    let user = user.update(db.deref()).await.unwrap();

    APIResponse::new(user)
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
