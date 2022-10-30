use sea_orm::{sea_query::Expr, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Value};
use teloxide::{
    prelude::*,
    types::{InlineKeyboardButton, MessageKind, ReplyMarkup, WebAppInfo},
    utils::command::BotCommands,
};
use url::Url;

pub async fn start_bot(bot: Bot, db: DatabaseConnection) {
    let handler = Update::filter_message()
        .filter_command::<Command>()
        .endpoint(commands_handler);

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![db])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await
}

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "🌧 Hello! These commands are supported:"
)]
enum Command {
    #[command(description = "show this message.")]
    Start,
    #[command(description = "purges all administrators of channel, except you.")]
    PurgeAdmins,
}

async fn commands_handler(
    bot: Bot,
    msg: Message,
    db: DatabaseConnection,
    cmd: Command,
) -> ResponseResult<()> {
    if let MessageKind::Common(common_message) = msg.kind {
        let user = match common_message.from {
            Some(u) => u,
            None => return Ok(()),
        };

        match cmd {
            Command::Start => {
                bot.send_message(msg.chat.id, Command::descriptions().to_string())
                    .reply_markup(ReplyMarkup::inline_kb([[InlineKeyboardButton::web_app(
                        "Open WebApp",
                        WebAppInfo {
                            url: Url::parse("https://webapp.ceheki.org/undefined.html").unwrap(),
                        },
                    )]]))
                    .await?;
            }
            Command::PurgeAdmins => {
                let model = entity::telegram_user::Entity::find_by_id(user.id.0 as i64)
                    .one(&db)
                    .await
                    .unwrap();
                let chat_id = match model {
                    Some(entity::telegram_user::Model {
                        channel: Some(channel),
                        ..
                    }) => channel,
                    _ => {
                        bot.send_message(msg.chat.id, "You don't have linked channel")
                            .await?;

                        return Ok(());
                    }
                };
                let member = bot.get_chat_member(ChatId(chat_id), user.id).await?;
                if !member.is_owner() {
                    bot.send_message(msg.chat.id, "You aren't the owner of linked chat")
                        .await?;

                    return Ok(());
                }

                let res = entity::telegram_user::Entity::update_many()
                    .col_expr(
                        entity::telegram_user::Column::Channel,
                        Expr::value(Value::String(None)),
                    )
                    .filter(entity::telegram_user::Column::Channel.eq(Some(chat_id)))
                    .filter(entity::telegram_user::Column::Id.ne(user.id.0 as i64))
                    .exec(&db)
                    .await
                    .unwrap();

                bot.send_message(
                    msg.chat.id,
                    format!("Purged {} administrators", res.rows_affected),
                )
                .await?;
            }
        };
    }

    Ok(())
}
