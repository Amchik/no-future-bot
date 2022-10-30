use sea_orm::DatabaseConnection;
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
}

async fn commands_handler(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    if let MessageKind::Common(_) = msg.kind {
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
        };
    }

    Ok(())
}
