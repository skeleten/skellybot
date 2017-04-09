#![recursion_limit = "2048"]

#[macro_use]
extern crate log;
extern crate env_logger;
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
mod message_handler;
pub mod handlers;
mod time;
pub mod schema;
pub mod models;

use error::*;
use context::*;
use diesel::prelude::*;
use dotenv::dotenv;
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

    env_logger::init().unwrap();

    let discord = Discord::from_bot_token(&token)
        .chain_err(|| "Login failed")?;

    let mut ctx = Context::new(discord);
    let mut mhs = message_handler::MessageHandlerStore::new();
    let handler_count = register_handlers(&mut mhs)?;
    info!("Registered {} handlers!", handler_count);

    let mut connection_tries = 0;
    while connection_tries < 5 {
        let (mut conn, re) = match ctx.client.connect().chain_err(|| "Failed to connect") {
            Ok(s) => {
                info!("Connected successfully");
                connection_tries = 0;
                s
            },
            Err(e) => {
                connection_tries += 1;
                if connection_tries >= 5 {
                    error!("Failed to connect 5 times, aborting");
                    return Err(e);
                } else {
                    warn!("Failed to connect, trying again {}/5", connection_tries);
                    continue;
                }
            },
        };

        let s = discord::State::new(re);
        ctx.set_state(s);

        loop {
            let event = match conn.recv_event().chain_err(|| "Failed to get event!") {
                Ok(e) => e,
                Err(e) => {
                    error!("Failed to get event: {:?}", e);
                    break;
                }
            };
            ctx.update_state(&event);
            match event {
                Event::MessageCreate(msg) => {
                    if let Err(e) = message_create_event(&mut ctx, &mhs, msg) {
                        error!("Error on MessageCreate: {:?}", e);
                    }
                },
                Event::ServerCreate(srv) => {
                    if let Err(e) = server_create_event(&mut ctx, srv) {
                        error!("Error on ServerCreate: {:?}", e);
                    }
                },
                Event::ReactionAdd(r) => ctx.user_seen(&r.user_id)?,
                Event::TypingStart { user_id, ..} => ctx.user_seen(&user_id)?,
                Event::PresenceUpdate { presence, ..} => {
                    if presence.status != OnlineStatus::Offline {
                        ctx.user_seen(&presence.user_id)?
                    }
                },

                _e => warn!("Unkown event"),
            }
        }
    }
    Ok(())
}

fn server_create_event(ctx: &mut Context, srv: PossibleServer<LiveServer>) -> Result<()> {
    if let PossibleServer::Online(srv) = srv {
        ctx.servers.push(srv.id);
        for p in srv.presences {
            if p.status != OnlineStatus::Offline {
                user_seen(ctx, &p.user_id)?;
            }
        }
    }
    Ok(())
}

fn message_create_event(ctx: &mut Context,
                        mhs: &message_handler::MessageHandlerStore,
                        msg: discord::model::Message) -> Result<()> {
    info!("Seen user by message: {}", msg.author.name);
    user_seen(ctx, &msg.author.id)?;
    process_message(msg, mhs, ctx)?;
    Ok(())
}

pub fn user_seen(ctx: &mut Context, uid: &UserId) -> Result<()> {
    use schema::users::dsl::*;
    debug!("updating user id {}", uid);
    let &UserId(uid) = uid;
    let uid = uid as i64;
    let conn = ctx.establish_connection()?;
    let results = users.filter(discord_id.eq(uid))
        .limit(1)
        .load::<models::User>(&conn)
        .chain_err(|| "Failed to load users")?;

    if results.len() >= 1 {
        let _user = diesel::update(users.find(id))
            .set(last_seen.eq(Some(std::time::SystemTime::now())))
            .get_result::<models::User>(&conn)
            .chain_err(|| "Failed to update user")?;
    } else {
        Context::create_user(&conn, uid, Some(std::time::SystemTime::now()))?;
    };
    Ok(())
}

pub fn process_message(mut msg: discord::model::Message,
                       mhs: &message_handler::MessageHandlerStore,
                       ctx: &mut Context) -> Result<()> {
    let ch = ctx.client.get_channel(msg.channel_id)?;
    let prefix: String = if let discord::model::Channel::Public(channel) = ch {
        match ctx.server_prefixes.get(&channel.server_id) {
            Some(p) => { p.to_string() },
            None => { context::DEFAULT_PREFIX.into() },
        }
    } else {
        context::DEFAULT_PREFIX.into()
    };

    let is_command = msg.content.starts_with(&prefix);
    if is_command {
        msg.content = msg.content.trim_left_matches(&prefix).into();
        mhs.call_handler(msg, ctx)?;
    }

    Ok(())
}

pub fn get_last_seen_time(ctx: &Context, did: u64) -> Result<Option<std::time::SystemTime>> {
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

/// Register known handlers.
/// Returns `Ok(NumberOfHandlersRegistered)` on success.

pub fn register_handlers(handler_store: &mut message_handler::MessageHandlerStore) -> Result<usize> {
    handler_store.register_handler("set-prefix", handlers::set_prefix);
    handler_store.register_handler("last-seen", handlers::last_seen);
    handler_store.register_handler("last-seen-all", handlers::last_seen_all);
    handler_store.register_handler("help", handlers::help);
    register_debug_handlers(handler_store)?;

    Ok(handler_store.get_handler_count())
}

/// Register debug handlers
#[cfg(feature = "debug")]

pub fn register_debug_handlers(handler_store: &mut message_handler::MessageHandlerStore) -> Result<()> {
    handler_store.register_handler("debug-print-prefixes", handlers::debug_print_prefixes);
    handler_store.register_handler("debug-error-test", handlers::debug_test_error);
    Ok(())
}

#[cfg(not(feature = "debug"))]

pub fn register_debug_handlers(handler_store: &mut message_handler::MessageHandlerStore) -> Result<()> {
    Ok(())
}
