use crate::models::{response::APIResponse, telegramauth::TelegramUser};

use itertools::Itertools;
use migration::{Condition, OnConflict};
use rocket::{delete, get, patch, put, routes, serde::json::Json, Route, State};
use sea_orm::{
    ActiveModelBehavior, ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait,
    ModelTrait, QueryFilter, QuerySelect, Set,
};
use serde::{Deserialize, Serialize};
use std::ops::Deref;

pub fn routes() -> Vec<Route> {
    routes![
        get_feed,
        patch_feed,
        get_scheduled_feed,
        create_scheduled_post,
        delete_scheduled_post
    ]
}

#[derive(Serialize)]
struct FeedElement<'a> {
    post: entity::post::Model,
    media: Vec<entity::post_media::Model>,
    author: &'a entity::author::Model,
}

#[derive(Serialize)]
struct ScheduledFeedElement {
    post: entity::scheduled_post::Model,
    media: Vec<entity::post_media::Model>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
enum FeedUpdateData<'a> {
    Subscribe(&'a str),
    Unsubscribe(&'a str),
    ReadUnder(i64),
}

#[derive(Deserialize)]
struct CreateScheduledPost {
    post_id: i64,
    post_text: Option<String>,
    #[serde(default)]
    exclude_media: Vec<i64>,
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

#[get("/scheduled")]
async fn get_scheduled_feed(
    db: &State<DatabaseConnection>,
    telegram_user: TelegramUser,
) -> APIResponse {
    let raw_posts = entity::scheduled_post::Entity::find()
        .filter(entity::scheduled_post::Column::UserId.eq(telegram_user.id))
        .all(db.deref())
        .await
        .unwrap();

    let mut posts = vec![];

    for post in raw_posts {
        let media_ids = post
            .media_ids
            .split(',')
            .flat_map(|f| f.parse::<i64>())
            .collect::<Vec<_>>();

        // NOTE: this code performs too many requests to db. May use
        // twitter-way (posts: Vec<_>, media: Vec<_>) or combine it later?
        // TODO:
        let media = if media_ids.is_empty() {
            vec![]
        } else {
            let mut conditions = Condition::any();
            for media in media_ids {
                conditions = conditions.add(entity::post_media::Column::Id.eq(media));
            }

            entity::post_media::Entity::find()
                .filter(conditions)
                .all(db.deref())
                .await
                .unwrap()
        };

        posts.push(ScheduledFeedElement { post, media });
    }

    APIResponse::new(posts)
}

#[put("/scheduled", data = "<data>")]
async fn create_scheduled_post(
    db: &State<DatabaseConnection>,
    telegram_user: TelegramUser,
    data: Json<CreateScheduledPost>,
) -> APIResponse {
    if data.0.exclude_media.len() > 8 {
        return APIResponse::error(422, "Excluded media too long");
    }

    let post = entity::post::Entity::find_by_id(data.post_id)
        .one(db.deref())
        .await
        .unwrap();
    let post = match post {
        Some(p) => p,
        _ => return APIResponse::error(404, "Post does not exists"),
    };

    let media_ids = {
        let mut cond = Condition::all();
        for media in data.0.exclude_media {
            cond = cond.add(entity::post_media::Column::Id.ne(media));
        }

        post.find_related(entity::post_media::Entity)
            .filter(cond)
            .all(db.deref())
            .await
            .unwrap()
            .into_iter()
            .map(|f| f.id)
            .join(",")
    };

    let post_text = data.0.post_text.unwrap_or(post.text);

    let active = entity::scheduled_post::ActiveModel {
        user_id: Set(telegram_user.id),
        media_ids: Set(media_ids),
        post_text: Set(post_text),
        post_source: Set(post.source_text),
        post_source_url: Set(post.source_url),
        ..Default::default()
    };

    let model = active.insert(db.deref()).await.unwrap();

    APIResponse::new(model)
}

#[delete("/scheduled/<id>")]
async fn delete_scheduled_post(
    db: &State<DatabaseConnection>,
    telegram_user: TelegramUser,
    id: u64,
) -> APIResponse {
    let id = id as i64;

    let result = entity::scheduled_post::Entity::delete_by_id(id)
        .filter(entity::scheduled_post::Column::UserId.eq(telegram_user.id))
        .exec(db.deref())
        .await
        .unwrap();

    if result.rows_affected != 0 {
        APIResponse::NoContent
    } else {
        APIResponse::error(404, "Post does not exists")
    }
}
