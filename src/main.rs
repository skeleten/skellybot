#![recursion_limit = "1024"]

extern crate redis;
#[macro_use]
extern crate error_chain;
extern crate discord;

mod error;

use error::*;

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
    let token = std::env::var("DISCORD_TOKEN")
        .chain_err(|| "DISCORD_TOKEN not set")?;

    let discord = Discord::from_bot_token(&token)
        .chain_err(|| "Login failed")?;

    let (mut conn, _) = discord.connect().chain_err(|| "Failed to connect")?;

    loop {
        let event = conn.recv_event().chain_err(|| "Failed to get event!")?;
        match event {
            Event::MessageCreate(msg)
                => println!("Received message {:#?}", msg),
            e => println!("Unkown event: {:#?}", e)
        }
    }
}
