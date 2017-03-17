#![recursion_limit = "2048"]

#[macro_use]
extern crate error_chain;
extern crate discord;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_codegen;
extern crate dotenv;

mod error;
mod context;

pub mod schema;
pub mod models;

use error::*;
use context::*;

use diesel::prelude::*;
use diesel::pg::PgConnection;
use dotenv::dotenv;
use std::env;

use discord::Discord;
use discord::model::Event;

fn main() {
    if let Err(ref e) = run() {
        println!("error: {}", e);
        for e in e.iter().skip(1) {
            println!("caused by: {}", e);
        }

        if let Some(backtrace) = e.backtrace() {
            println!("backtrace: {:?}", backtrace);
        }

        ::std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let mut ctx = Context::new();
    match dotenv() {
        Ok(_) => { },
        Err(e) => bail!("Failed to init env: {:?}", e),
    };

    let token = std::env::var("DISCORD_TOKEN")
        .chain_err(|| "DISCORD_TOKEN not set")?;

    let discord = Discord::from_bot_token(&token)
        .chain_err(|| "Login failed")?;

    let mut connection_tries = 0;
    while connection_tries < 5 {
        let (mut conn, _) = match discord.connect().chain_err(|| "Failed to connect") {
            Ok(s) => {
                connection_tries = 0;
                s
            },
            Err(e) => {
                connection_tries += 1;
                if connection_tries >= 5 {
                    println!("Failed to connect 5 times, aborting");
                    return Err(e);
                } else {
                    println!("Failed to connect, trying again {}/5", connection_tries);
                    continue;
                }
            },
        };

        loop {
            let event = match conn.recv_event().chain_err(|| "Failed to get event!") {
                Ok(e) => e,
                Err(e) => {
                    println!("Failed to get event: {:?}", e);
                    break;
                }
            };
            match event {
                Event::MessageCreate(msg) => message_create_event(&mut ctx, msg)?,
                e => println!("Unkown event"),
            }
        }
    }
    Ok(())
}

fn message_create_event(ctx: &mut Context, msg: discord::model::Message) -> Result<()> {
    println!("Seen user by message: {}", msg.author.name);
    user_seen(ctx, &msg.author)?;
    // TODO: Handle commands
    Ok(())
}

pub fn user_seen(ctx: &mut Context, user: &discord::model::User) -> Result<()> {
    use schema::users::dsl::*;
    let discord::model::UserId(uid) = user.id;
    let uid = uid as i64;
    let conn = establish_connection()?;
    let results = users.filter(discord_id.eq(uid))
        .limit(1)
        .load::<models::User>(&conn)
        .chain_err(|| "Failed to load users")?;

    if results.len() >= 1 {
        let dbuid = results[0].id;
        let user = diesel::update(users.find(id))
            .set(last_seen.eq(Some(std::time::SystemTime::now())))
            .get_result::<models::User>(&conn)
            .chain_err(|| "Failed to update user")?;
    } else {
        create_user(&conn, uid, Some(std::time::SystemTime::now()))?;
    };
    Ok(())
}

pub fn establish_connection() -> Result<PgConnection> {
    let db_url = env::var("DATABASE_URL")
        .chain_err(|| "DATABASE_URL must be set!")?;
    PgConnection::establish(&db_url)
        .chain_err(|| format!("Error connecting to {}", db_url))
}

pub fn create_user(conn: &PgConnection, discord_id: i64, last_seen: Option<std::time::SystemTime>) -> Result<models::User> {
    use schema::users;

    let new_user = models::NewUser {
        discord_id: discord_id,
        last_seen: last_seen,
    };

    diesel::insert(&new_user)
        .into(users::table)
        .get_result(conn)
        .chain_err(|| "Error saving new user")
}
