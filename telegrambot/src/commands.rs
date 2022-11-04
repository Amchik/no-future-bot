use sea_orm::{sea_query::OnConflict, DatabaseConnection, EntityTrait, Set};
use teloxide::{requests::Requester, types::ChatId, Bot};

#[derive(Debug, Clone, Copy)]
#[repr(u16)]
pub enum LinkChannelError {
    NotFound = 404,
    InvalidFormat = 400,
    BotNotAdmin = 401,
    UserNoPermissions = 403,
}
impl std::fmt::Display for LinkChannelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(f, "Channel not found"),
            Self::InvalidFormat => write!(f, "Channel ID has invalid format"),
            Self::BotNotAdmin => write!(f, "Bot is not chat administrator"),
            Self::UserNoPermissions => write!(
                f,
                "You need to be a chat owner or channel administrator to do that",
            ),
        }
    }
}
impl std::error::Error for LinkChannelError {}

pub async fn link_channel(
    user_id: i64,
    channel_id: &str,
    bot: &Bot,
    db: &DatabaseConnection,
) -> Result<entity::telegram_user::Model, LinkChannelError> {
    let chat_id = match channel_id.parse::<i64>() {
        Ok(c) => c,
        _ if channel_id.starts_with('@') => {
            let chat_id = channel_id;

            match bot.get_chat(chat_id.to_string()).await {
                Ok(chat) => chat.id.0,
                _ => return Err(LinkChannelError::NotFound),
            }
        }
        _ => return Err(LinkChannelError::InvalidFormat),
    };

    let members = match bot.get_chat_administrators(ChatId(chat_id)).await {
        Ok(m) => m,
        _ => return Err(LinkChannelError::BotNotAdmin),
    };
    if !members
        .iter()
        .any(|f| f.user.id.0 == user_id as u64 && f.can_post_messages())
    {
        return Err(LinkChannelError::UserNoPermissions);
    }

    // assume bot can post messages...

    let active = entity::telegram_user::ActiveModel {
        id: Set(user_id),
        channel: Set(Some(chat_id)),
        ..Default::default()
    };
    let user = entity::telegram_user::Entity::insert(active)
        .on_conflict(
            OnConflict::column(entity::telegram_user::Column::Id)
                .update_column(entity::telegram_user::Column::Channel)
                .to_owned(),
        )
        .exec_with_returning(db)
        .await
        .unwrap();

    Ok(user)
}
