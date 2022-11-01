use crate::models::{response::APIResponse, telegramauth::TelegramUser};

use migration::OnConflict;
use rocket::{get, patch, routes, serde::json::Json, Route, State};
use sea_orm::{
    ActiveModelBehavior, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QuerySelect,
    Set,
};
use serde::{Deserialize, Serialize};
use std::ops::Deref;

pub fn routes() -> Vec<Route> {
    routes![get_feed, patch_feed]
}

#[derive(Serialize)]
struct FeedElement<'a> {
    post: entity::post::Model,
    media: Vec<entity::post_media::Model>,
    author: &'a entity::author::Model,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
enum FeedUpdateData<'a> {
    Subscribe(&'a str),
    Unsubscribe(&'a str),
    ReadUnder(i64),
}

#[get("/")]
async fn get_feed(db: &State<DatabaseConnection>, telegram_user: TelegramUser) -> APIResponse {
    let user = entity::telegram_user::Entity::find_by_id(telegram_user.id)
        .one(db.deref())
        .await
        .unwrap();

    let user = if let Some(u) = user {
        u
    } else {
        return APIResponse::new::<Vec<()>>(vec![]);
    };

    let follows = entity::follow::Entity::find()
        .filter(entity::follow::Column::UserId.eq(user.id))
        .find_also_related(entity::author::Entity)
        .all(db.deref())
        .await
        .unwrap();

    let mut new_posts = vec![];

    for author in follows.iter().flat_map(|v| &v.1) {
        let posts = entity::post::Entity::find()
            .filter(entity::post::Column::AuthorId.eq(author.id))
            .filter(entity::post::Column::Id.gt(user.last_feed_id))
            .find_with_related(entity::post_media::Entity)
            .limit(15)
            .all(db.deref())
            .await
            .unwrap();

        for (post, media) in posts {
            new_posts.push(FeedElement {
                post,
                media,
                author,
            })
        }
    }

    new_posts.sort_by_key(|f| f.post.platform_id);

    APIResponse::new(new_posts)
}

#[patch("/", data = "<data>")]
async fn patch_feed(
    db: &State<DatabaseConnection>,
    telegram_user: TelegramUser,
    data: Json<FeedUpdateData<'_>>,
) -> APIResponse {
    if let FeedUpdateData::ReadUnder(id) = data.0 {
        let mut active = entity::telegram_user::ActiveModel::new();
        active.id = Set(telegram_user.id);
        active.last_feed_id = Set(id);

        entity::telegram_user::Entity::insert(active)
            .on_conflict(
                OnConflict::column(entity::telegram_user::Column::Id)
                    .update_column(entity::telegram_user::Column::LastFeedId)
                    .to_owned(),
            )
            .exec(db.deref())
            .await
            .unwrap();

        return APIResponse::NoContent;
    }

    let author_id = match data.0 {
        FeedUpdateData::Subscribe(s) | FeedUpdateData::Unsubscribe(s) => s,
        _ => unreachable!(),
    };

    let author_stmt = match author_id.parse::<i64>() {
        Ok(id) if id > 0 => {
            entity::author::Entity::find().filter(entity::author::Column::PlatformId.eq(id))
        }
        Ok(id) if id < 0 => entity::author::Entity::find_by_id(-id),
        _ => entity::author::Entity::find().filter(entity::author::Column::Username.eq(author_id)),
    };

    let author = match author_stmt.one(db.deref()).await {
        Ok(Some(a)) => a,
        Ok(None) => return APIResponse::error(404, "Author does not exists"),
        Err(e) => panic!("{e}"),
    };

    let active = entity::follow::ActiveModel {
        user_id: Set(telegram_user.id),
        author_id: Set(author.id),
    };

    let is_ok = match data.0 {
        FeedUpdateData::Subscribe(_) => entity::follow::Entity::insert(active)
            .exec(db.deref())
            .await
            .is_ok(),
        FeedUpdateData::Unsubscribe(_) => entity::follow::Entity::delete(active)
            .exec(db.deref())
            .await
            .is_ok(),
        _ => unreachable!(),
    };

    if is_ok {
        APIResponse::NoContent
    } else {
        APIResponse::error(400, "Invalid operation")
    }
}
