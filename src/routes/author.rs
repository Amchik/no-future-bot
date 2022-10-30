use entity::telegram_user::POWER_MOD;
use migration::OnConflict;
use rocket::{get, put, routes, Route, State};
use sea_orm::{
    ActiveModelBehavior, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
    TryIntoModel,
};
use serde::Serialize;
use std::ops::Deref;

use crate::models::{
    response::APIResponse, telegramauth::TelegramUser, twitterclient::TwitterClient,
};

pub fn routes() -> Vec<Route> {
    routes![
        get_by_platform_id,
        get_by_username,
        put_by_platform_id,
        put_by_username,
        get_posts_by_platform_id,
        get_posts_by_username
    ]
}

#[derive(Serialize)]
struct PostData {
    post: entity::post::Model,
    media: Vec<entity::post_media::Model>,
}

#[get("/<id>")]
async fn get_by_platform_id(id: i64, db: &State<DatabaseConnection>) -> APIResponse {
    let expr = if id >= 0 {
        entity::author::Entity::find().filter(entity::author::Column::PlatformId.eq(id))
    } else {
        entity::author::Entity::find_by_id(-id)
    };

    let author = expr.one(db.deref()).await.unwrap();

    match author {
        Some(a) => APIResponse::new(a),
        None => APIResponse::error(404, "Author does not exists"),
    }
}

#[get("/<id>", rank = 2)]
async fn get_by_username(id: &str, db: &State<DatabaseConnection>) -> APIResponse {
    let author = entity::author::Entity::find()
        .filter(entity::author::Column::Username.eq(id))
        .one(db.deref())
        .await
        .unwrap();

    match author {
        Some(a) => APIResponse::new(a),
        None => APIResponse::error(404, "Author does not exists"),
    }
}

#[put("/<id>")]
async fn put_by_platform_id(
    id: i64,
    db: &State<DatabaseConnection>,
    twitter_client: &State<TwitterClient>,
    telegram_user: TelegramUser,
) -> APIResponse {
    if id <= 0 {
        return APIResponse::error(422, "id must be positive integer");
    }

    let user_is_mod = entity::telegram_user::Entity::find_by_id(telegram_user.id)
        .one(db.deref())
        .await
        .unwrap()
        .map_or(false, |v| v.power_level >= POWER_MOD);

    if !user_is_mod {
        return APIResponse::error(403, format!("You need power level {POWER_MOD} or higher"));
    }

    let author = match twitter_client.fetch_user(id as u64).await {
        Ok(a) => a,
        _ => return APIResponse::error(404, "Twitter user not found"),
    };

    let mut active = entity::author::ActiveModel::new();
    active.platform_id = Set(author.id);
    active.name = Set(author.name);
    active.username = Set(author.username);
    active.avatar_url = Set(author.profile_image_url);

    let id = entity::author::Entity::insert(active.clone())
        .on_conflict(
            OnConflict::column(entity::author::Column::PlatformId)
                .update_columns([
                    entity::author::Column::Name,
                    entity::author::Column::Username,
                    entity::author::Column::AvatarUrl,
                ])
                .to_owned(),
        )
        .exec(db.deref())
        .await
        .unwrap()
        .last_insert_id;

    active.id = Set(id);

    APIResponse::new(active.try_into_model().unwrap())
}

#[put("/<id>", rank = 2)]
async fn put_by_username(
    id: &str,
    db: &State<DatabaseConnection>,
    twitter_client: &State<TwitterClient>,
    telegram_user: TelegramUser,
) -> APIResponse {
    let user_is_mod = entity::telegram_user::Entity::find_by_id(telegram_user.id)
        .one(db.deref())
        .await
        .unwrap()
        .map_or(false, |v| v.power_level >= POWER_MOD);

    if !user_is_mod {
        return APIResponse::error(403, format!("You need power level {POWER_MOD} or higher"));
    }

    let author = match twitter_client.fetch_user_by_username(id).await {
        Ok(a) => a,
        _ => return APIResponse::error(404, "Twitter user not found"),
    };

    let mut active = entity::author::ActiveModel::new();
    active.platform_id = Set(author.id);
    active.name = Set(author.name);
    active.username = Set(author.username);
    active.avatar_url = Set(author.profile_image_url);

    let id = entity::author::Entity::insert(active.clone())
        .on_conflict(
            OnConflict::column(entity::author::Column::PlatformId)
                .update_columns([
                    entity::author::Column::Name,
                    entity::author::Column::Username,
                    entity::author::Column::AvatarUrl,
                ])
                .to_owned(),
        )
        .exec(db.deref())
        .await
        .unwrap()
        .last_insert_id;

    active.id = Set(id);

    APIResponse::new(active.try_into_model().unwrap())
}

#[get("/<id>/posts")]
async fn get_posts_by_platform_id(
    id: i64,
    db: &State<DatabaseConnection>,
    _telegram_user: TelegramUser,
) -> APIResponse {
    let author_id = if id >= 0 {
        let author = entity::author::Entity::find()
            .filter(entity::author::Column::PlatformId.eq(id))
            .one(db.deref())
            .await
            .unwrap();

        match author {
            Some(a) => a.id,
            None => return APIResponse::error(404, "Author does not exists"),
        }
    } else {
        -id
    };

    let posts = entity::post::Entity::find()
        .filter(entity::post::Column::AuthorId.eq(author_id))
        .find_with_related(entity::post_media::Entity)
        .all(db.deref())
        .await
        .unwrap()
        .into_iter()
        .map(|(post, media)| PostData { post, media })
        .collect::<Vec<PostData>>();

    APIResponse::new(posts)
}

#[get("/<id>/posts", rank = 2)]
async fn get_posts_by_username(
    id: &str,
    db: &State<DatabaseConnection>,
    _telegram_user: TelegramUser,
) -> APIResponse {
    let author = entity::author::Entity::find()
        .filter(entity::author::Column::Username.eq(id))
        .one(db.deref())
        .await
        .unwrap();

    let author_id = match author {
        Some(a) => a.id,
        None => return APIResponse::error(404, "Author does not exists"),
    };

    let posts = entity::post::Entity::find()
        .filter(entity::post::Column::AuthorId.eq(author_id))
        .find_with_related(entity::post_media::Entity)
        .all(db.deref())
        .await
        .unwrap()
        .into_iter()
        .map(|(post, media)| PostData { post, media })
        .collect::<Vec<PostData>>();

    APIResponse::new(posts)
}
