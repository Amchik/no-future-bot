use std::ops::Deref;
use std::time::Duration;

use entity::post_media::MediaType;
use migration::Condition;
use reqwest::Url;
use rocket::tokio::time::sleep;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use telegrambot::teloxide::{
    payloads::SendMessageSetters,
    requests::Requester,
    types::{ChatId, InputFile, InputMedia, InputMediaPhoto, InputMediaVideo, ParseMode},
    Bot, RequestError,
};

fn escape_html(text: &str) -> String {
    let mut buff = String::with_capacity(text.len());

    for c in text.chars() {
        match c {
            '<' => buff.push_str("&lt;"),
            '>' => buff.push_str("&gt;"),
            '&' => buff.push_str("&amp;"),
            _ => buff.push(c),
        }
    }

    buff
}

pub async fn start_posting_worker(db: &DatabaseConnection, bot: &Bot) {
    loop {
        let posts = entity::scheduled_post::Entity::find()
            .find_also_related(entity::telegram_user::Entity)
            .all(db.deref())
            .await
            .unwrap()
            .into_iter()
            .filter(|f| f.1.is_some())
            .map(|f| (f.0, f.1.unwrap()))
            .filter(|f| f.1.channel.is_some());

        let mut del_cond = Condition::any();

        for (post, user) in posts {
            let chat_id = ChatId(user.channel.unwrap());

            let text = format!(
                "{}\n\n<b><a href=\"{}\">{}</a></b>",
                escape_html(&post.post_text),
                post.post_source_url,
                escape_html(&post.post_source)
            );

            let media_ids = post
                .media_ids
                .split(',')
                .flat_map(|f| f.parse::<i64>())
                .collect::<Vec<_>>();

            let err = if !media_ids.is_empty() {
                let mut cond = Condition::any();
                for media in media_ids {
                    cond = cond.add(entity::post_media::Column::Id.eq(media));
                }
                let mut media = entity::post_media::Entity::find()
                    .filter(cond)
                    .all(db.deref())
                    .await
                    .unwrap()
                    .into_iter()
                    .map(|f| match f.media_type {
                        MediaType::Photo => InputMedia::Photo(InputMediaPhoto::new(
                            InputFile::url(Url::parse(&f.media_url).unwrap()),
                        )),
                        MediaType::Video => InputMedia::Video(InputMediaVideo::new(
                            InputFile::url(Url::parse(&f.media_url).unwrap()),
                        )),
                    })
                    .collect::<Vec<_>>();

                // little trolling, but
                // <@nanoqsh> Зато DRY :molodec:
                if let Some(
                    InputMedia::Photo(InputMediaPhoto {
                        caption,
                        parse_mode,
                        ..
                    })
                    | InputMedia::Video(InputMediaVideo {
                        caption,
                        parse_mode,
                        ..
                    }),
                ) = media.get_mut(0)
                {
                    *caption = Some(text);
                    *parse_mode = Some(ParseMode::Html);
                }

                bot.send_media_group(chat_id, media).await.err()
            } else {
                bot.send_message(chat_id, text)
                    .parse_mode(ParseMode::Html)
                    .await
                    .err()
            };

            if let Some(err) = err {
                match err {
                    RequestError::RetryAfter(d) => sleep(d).await,
                    _ => eprintln!("Failed to post message: {err}"),
                }
            }

            del_cond = del_cond.add(entity::scheduled_post::Column::Id.eq(post.id));
        }

        entity::scheduled_post::Entity::delete_many()
            .filter(del_cond)
            .exec(db.deref())
            .await
            .unwrap();

        sleep(Duration::from_secs(120)).await;
    }
}
