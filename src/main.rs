#![recursion_limit = "2048"]

#[macro_use]
extern crate error_chain;
extern crate discord;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_codegen;
extern crate dotenv;
extern crate chrono;

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
use chrono::datetime::*;

use discord::Discord;
use discord::model::*;

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
    match dotenv() {
        Ok(_) => { },
        Err(e) => bail!("Failed to init env: {:?}", e),
    };

    let token = std::env::var("DISCORD_TOKEN")
        .chain_err(|| "DISCORD_TOKEN not set")?;

    let discord = Discord::from_bot_token(&token)
        .chain_err(|| "Login failed")?;

    let mut ctx = Context::new(discord);

    let mut connection_tries = 0;
    while connection_tries < 5 {
        let (mut conn, _) = match ctx.client.connect().chain_err(|| "Failed to connect") {
            Ok(s) => {
                println!("Connected successfully");
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
                Event::ServerCreate(srv) => server_create_event(&mut ctx, srv)?,
                e => println!("Unkown event"),
            }
        }
    }
    Ok(())
}

fn server_create_event(ctx: &mut Context, srv: PossibleServer<LiveServer>) -> Result<()> {
    if let PossibleServer::Online(srv) = srv {
        ctx.servers.push(srv.id);
        for p in srv.presences {
            let p: Presence = p;
            if p.status != OnlineStatus::Online {
                user_seen(ctx, &p.user_id);
            }
        }
    }
    Ok(())
}

fn message_create_event(ctx: &mut Context, msg: discord::model::Message) -> Result<()> {
    println!("Seen user by message: {}", msg.author.name);
    user_seen(ctx, &msg.author.id)?;
    process_message(ctx, &msg)?;
    Ok(())
}

pub fn user_seen(ctx: &mut Context, uid: &UserId) -> Result<()> {
    use schema::users::dsl::*;
    println!("updating user id {}", uid);
    let &UserId(uid) = uid;
    let uid = uid as i64;
    let conn = ctx.establish_connection()?;
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
        Context::create_user(&conn, uid, Some(std::time::SystemTime::now()))?;
    };
    Ok(())
}

pub fn process_message(ctx: &mut Context, msg: &discord::model::Message) -> Result<()> {
    let m = &msg.content;

    if m.starts_with("!last_seen") {
        process_message_last_seen(ctx, msg)?;
    }

    Ok(())
}

pub fn process_message_last_seen(ctx: &mut Context, msg: &Message) -> Result<()> {
    println!("processing_message_last_seen");
    let mut message = "".to_string();
    for m in msg.mentions.iter() {
        let UserId(id) = m.id;
        let mut user = None;
        for sid in ctx.servers.iter() {
            let get_user_result = ctx.client.get_member(sid.clone(), m.id);
            match get_user_result {
                Ok(u) => {
                    user = Some(u);
                    break;
                },
                Err(_) => {
                    println!("Couldnt get {:?} from {:?}", sid, m.id);
                }
            }
        }
        let time = get_last_seen_time(ctx, id)?;
        let user_ref = if let Some(u) = user {
            if let Some(nick) = u.nick {
                nick
            } else {
                u.user.name
            }
        } else {
            format!("UID {}", id).to_string()
        };
        let time_ref = if let Some(t) = time {
            let t = system_time_to_date_time(t);
            use chrono::{Datelike, Timelike};
            format!("{:04}-{:02}-{:02} {:02}:{:02} UTC",
                    t.year(),
                    t.month(),
                    t.day(),
                    t.hour(),
                    t.minute())
        } else {
            format!("never")
        };

        message.push_str(&format!("{} last seen: {}", user_ref, time_ref));
    }

    if message.len() > 0 {
        ctx.client.send_message(&msg.channel_id, &message, "", false)?;
    }

    Ok(())
}

fn system_time_to_date_time(t: std::time::SystemTime) -> DateTime<chrono::offset::utc::UTC> {
    let (sec, nsec) = match t.duration_since(std::time::UNIX_EPOCH) {
        Ok(dur) => (dur.as_secs() as i64, dur.subsec_nanos()),
        Err(e) => { // unlikely but should be handled
            let dur = e.duration();
            let (sec, nsec) = (dur.as_secs() as i64, dur.subsec_nanos());
            if nsec == 0 {
                (-sec, 0)
            } else {
                (-sec - 1, 1_000_000_000 - nsec)
            }
        },
    };
    use chrono::TimeZone;
    chrono::offset::utc::UTC.timestamp(sec, nsec)
}

pub fn get_last_seen_time(ctx: &mut Context, did: u64) -> Result<Option<std::time::SystemTime>> {
    use schema::users::dsl::*;
    let conn = ctx.establish_connection()?;
    let results = users
        .filter(discord_id.eq(did as i64))
        .limit(1)
        .load::<models::User>(&conn)
        .chain_err(|| "Could not load users!")?;

    if results.len() == 0 {
        Ok(None)
    } else {
        let u: &models::User = &results[0];
        Ok(u.last_seen)
    }
}
