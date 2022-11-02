use std::{fs::File, io, process::exit};

use clap::Parser;
use migration::{Migrator, MigratorTrait};
use rocket::tokio::select;
use sea_orm::Database;
use telegrambot::{start_bot, teloxide::Bot};

use crate::{
    models::twitterclient::TwitterClient,
    workers::{posting::start_posting_worker, twitter::start_twitter_collector},
};

mod models;
mod routes;
mod workers;

#[derive(Parser)]
#[clap(version, about, long_about = None)]
struct Args {
    #[clap(long, default_value = "no-future-bot.db")]
    /// Database path
    sqlite: String,

    #[clap(long)]
    /// Run migrations and exit
    migrate_only: bool,

    #[clap(short = 'b', long)]
    /// Telegram bot token
    telegram_token: Option<String>,

    #[clap(short, long)]
    /// Twitter bearer token
    twitter_token: Option<String>,
}

#[rocket::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    {
        let file = File::options()
            .write(true)
            .create_new(true)
            .open(&args.sqlite);
        match file {
            Ok(_) => {}
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
            Err(e) => panic!("Failed to open file {}: {e}", args.sqlite),
        }
    }

    let db = Database::connect(format!("sqlite://{}", args.sqlite)).await?;
    Migrator::up(&db, None).await?;

    let bot = Bot::new(args.telegram_token.unwrap_or_else(|| {
        std::env::var("TELEGRAM_TOKEN").unwrap_or_else(|_| {
            eprintln!(
                "Please set $TELEGRAM_TOKEN env var, or provide it using --telegram-token key."
            );

            exit(1)
        })
    }));

    let twitter = TwitterClient::new(args.twitter_token.unwrap_or_else(|| {
        std::env::var("TWITTER_TOKEN").unwrap_or_else(|_| {
            eprintln!(
                "Please set $TWITTER_TOKEN env var, or provide it using --twitter-token key."
            );

            exit(1)
        })
    }));

    if args.migrate_only {
        exit(0);
    }

    let rocket = rocket::build()
        .configure(rocket::Config {
            keep_alive: 0,
            ident: rocket::config::Ident::none(),
            ..Default::default()
        })
        .manage(db.clone())
        .manage(bot.clone())
        .manage(twitter.clone())
        .register("/", routes::catchers::catchers())
        .mount("/user", routes::user::routes())
        .mount("/author", routes::author::routes())
        .mount("/feed", routes::feed::routes())
        .launch();

    let telegram_bot = start_bot(bot.clone(), db.clone());

    let twitter_worker = start_twitter_collector(&db, &twitter);

    let telegram_worker = start_posting_worker(&db, &bot);

    select! {
        res = rocket => { let _ = res.unwrap(); },
        () = telegram_bot => (),
        () = twitter_worker => (),
        () = telegram_worker => (),
    };

    Ok(())
}
