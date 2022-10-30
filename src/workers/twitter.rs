use std::time::Duration;

use rocket::tokio::time::sleep;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};

use crate::models::twitterclient::TwitterClient;

pub async fn start_twitter_collector(db: &DatabaseConnection, twitter: &TwitterClient) {
    loop {
        let authors = entity::author::Entity::find().all(db).await.unwrap();

        for author in authors {
            let last_post = entity::post::Entity::find()
                .filter(entity::post::Column::AuthorId.eq(author.id))
                .order_by_desc(entity::post::Column::PlatformId)
                .one(db)
                .await
                .unwrap()
                .map(|f| f.platform_id);

            let id = author.platform_id.to_string();

            let new_posts = match twitter.fetch_timeline(&id, last_post).await {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("Warning: twitter.fetch_timeline of {id} failed: {e}");

                    continue;
                }
            };

            for post in new_posts {
                let active = entity::post::ActiveModel {
                    platform_id: Set(post.id),
                    author_id: Set(author.id),
                    text: Set(post.text),
                    source_text: Set(post.author_name),
                    source_url: Set(format!(
                        "https://twitter.com/{}/status/{}",
                        post.author_username, post.id
                    )),
                    ..Default::default()
                };
                let model = active.insert(db).await.unwrap();

                let media = post
                    .media
                    .into_iter()
                    .map(|f| entity::post_media::ActiveModel {
                        post_id: Set(model.id),
                        media_type: Set(f.media_type()),
                        media_url: Set(f.media_url()),
                        ..Default::default()
                    })
                    .collect::<Vec<_>>();

                entity::post_media::Entity::insert_many(media)
                    .exec(db)
                    .await
                    .unwrap();
            }
        }

        sleep(Duration::from_secs(120)).await;
    }
}
