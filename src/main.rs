use std::{
    env,
    error,
};

use serenity::{
    http,
    model::webhook::Webhook,
    model::channel::Message,
};

struct DiscordBot {
    webhook: Webhook,
}

impl DiscordBot {
    pub fn new() -> Result<DiscordBot, Box<error::Error>> {
        let id: u64 = env::var("DISCORD_ID")
                        .expect("set DISCORD_ID")
                        .parse()?;
        let token = &env::var("DISCORD_TOKEN").expect("set DISCORD_TOKEN");
        let webhook = http::get_webhook_with_token(id, token)?;
        Ok(DiscordBot {
            webhook
        })
    }

    pub fn msg(&self, msg: &str) -> Result<Option<Message>, Box<error::Error>> {
        let msg = self.webhook.execute(/*wait?*/ false, |w| w.content(msg))?;
        Ok(msg)
    }
}

fn main() {
    match main2() {
        Ok(()) => {},
        Err(err) => eprintln!("{:#?}", err),
    }
}

fn main2() ->  Result<(), Box<error::Error>> {
    let bot = DiscordBot::new()?;
    bot.msg("`DiscordBot::new()` ✓")?;

    Ok(())
}
