use ::context::*;
use ::discord::model::*;
use ::error::*;

/// Handles a message to set the new prefix for a server
///
/// The format of the message is
/// ```
/// [PREFIX]set-prefix [NEW-PREFIX]
/// ```
/// Where `NEW-PREFIX` may not contain any spaces.
/// everything after `NEW-PREFIX` is ignored.

pub fn set_prefix(msg: &Message, ctx: &mut Context) -> Result<bool> {
    // Format of the message is
    // [PREFIX]set-prefix [NEW-PREFIX]
    let parts = msg.content.trim().split_whitespace().collect::<Vec<_>>();
    if parts.len() < 2 {
        // we did not get enough arguments
        bail!("Not enough Arguments! Usage: `set-prefix [NEW-PREFIX]`");
    } else {
        let channel = ctx.client.get_channel(msg.channel_id)?;
        if let Channel::Public(ch) = channel {
            let new_prefix = parts[1].trim();
            ctx.server_prefixes.insert(ch.server_id, new_prefix.into());
            ctx.client.send_message(&msg.channel_id, &format!("Set prefix to `{}`!", new_prefix), "", false)?;
        } else {
            bail!("This command can only be used on a server.")
        }
    }

    Ok(true)
}

/// Handles a message to get the last seen times for a list of users
///
/// The format of the message is
/// ```
/// [PREFIX]last-seen [MENTION..]
/// ```
///
/// Where `MENTION..` is a list of mentions. `@here` and `@everyone` are
/// currently not supported.
///
/// Any text outside of those mentions will be ignored

pub fn last_seen(msg: &Message, ctx: &mut Context) -> Result<bool> {
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
                    warn!("Couldnt get {:?} from {:?}", sid, m.id);
                }
            }
        }
        let time = ::get_last_seen_time(ctx, id)?;
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
            ::time::format_time(t)
        } else {
            format!("never")
        };

        message.push_str(&format!("{} last seen: {}", user_ref, time_ref));
    }

    if message.len() > 0 {
        ctx.client.send_message(&msg.channel_id, &message, "", false)?;
    }

    Ok(true)
}

/// Handles a message to get the last seen times of all users on a server
///
/// The format of the message is
/// ```
/// [PREFIX]last-seen-all
/// ```
///
/// Any trailing text will be ignored.

pub fn last_seen_all(msg: &Message, ctx: &mut Context) -> Result<bool> {
    let mut message = "```\n".to_string();

    let channel = ctx.client.get_channel(msg.channel_id)?;
    let channel = if let Channel::Public(ch) = channel {
        ch
    } else {
        bail!("Can only be used on servers!");
    };
    let server = ctx.state.as_ref().map(|st| st.servers()
                                        .iter()
                                        .filter(|s| s.id == channel.server_id)
                                        .nth(0));
    let server = match server {
        Some(Some(s)) => s,
        _ => bail!("Can only be used on servers!"),
    };
    for m in server.members.iter() {
        let m: &Member = m;
        let name = m.display_name();
        let UserId(id) = m.user.id;
        let time = match ::get_last_seen_time(&ctx, id) {
            Ok(t) => t,
            _ => bail!("Couldn't get time!")
        };
        let time_ref = if let Some(t) = time {
            ::time::format_time(t)
        } else {
            format!("never")
        };
        message += &format!("{:20}\t{}\n", name, time_ref);
    }
    message += "\n```";

    if message.trim() != "```\n\n```" {
        ctx.client.send_message(&msg.channel_id, &message, "", false)?;
    }

    Ok(true)
}

/// Handles a message that prompts the help text.
///
/// The format of the message is
/// ```
/// [PREFIX]help
/// ```
///
/// Any additional trailing text will be ignored

pub fn help(msg: &Message, ctx: &mut Context) -> Result<bool> {
    let msg_text = include_str!("../include/help.md");
    ctx.client.send_message(&msg.channel_id, msg_text, "", false)?;
    Ok(true)
}

/// Handles a message to print all prefix-overwrites
///
/// The format of the message is
/// ```
/// [PREFIX]debug-print-prefixes
/// ```
///
/// Any trailing text will be ignored.
/// Only available if compiled with the `debug`-feature
#[cfg(feature = "debug")]

pub fn debug_print_prefixes(msg: &Message, ctx: &mut Context) -> Result<bool> {
    let mut m = String::new();
    m += "Server ID\tPrefix\n\n";
    for (s, p) in ctx.server_prefixes.iter() {
        let &ServerId(id) = s;
        m += &format!("{}\t`{}`", id, p);
    };

    ctx.client.send_message(&msg.channel_id, &m, "", false)?;

    Ok(true)
}

/// Handles message to prompt a test of the error-handling.
///
/// The format of the message is
/// ```
/// [PREFIX]debug-error-test
/// ```
///
/// Any trailing text will be ignored.
/// Only available if compiled with the `debug`-feature
#[cfg(feature = "debug")]

pub fn debug_test_error(_msg: &Message, _ctx: &mut Context) -> Result<bool> {
    bail!("Error-Test!");
}
