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
            let t = ::system_time_to_date_time(t);
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

    Ok(true)
}

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

pub fn debug_test_error(_msg: &Message, _ctx: &mut Context) -> Result<bool> {
    bail!("Error-Test!");
}
