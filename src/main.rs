use std::{
    env,
    error,
    fmt,
    str,
};

use reqwest::{
    Client,
    Url,
};

use serenity::{
    http,
    model::webhook::Webhook,
    model::channel::Message,
};

mod robinhood;

// The maximum number of characters allowed in a Discord message.
// Updated: October 28, 2018
const DISCORD_MAX_MSG_CHAR_LEN: usize = 2000;

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

    // TODO: Impl Write?
    pub fn write(&self, msg: &str) -> Result<Option<Message>, Box<error::Error>> {

        let mut msg = msg;
        // Discord has a maximum message size.
        // It will reject messages that are too long, and we can shorten them here.
        if msg.len() > DISCORD_MAX_MSG_CHAR_LEN {
            eprintln!("WARNING: Discord message is too long. len={}, max={}",
                      msg.len(),
                      DISCORD_MAX_MSG_CHAR_LEN);
            // Truncating a message using the "```" formatters looks *awful*.
            // So we'll let that one go through (and fail).
            if !msg.contains("```") {
                msg = &msg[..DISCORD_MAX_MSG_CHAR_LEN];
            } else {
                eprintln!("WARNING: Message contains \"```\" - it will sent without being shortened");
            }
        }
        Ok(self.webhook.execute(/*wait?*/ false, |w| w.content(msg))?)
    }

    // TODO: Impl Write?
    pub fn writef(&self, args: fmt::Arguments) -> Result<Option<Message>, Box<error::Error>> {
        let utf8 = format!("{}", args);
        self.write(&utf8)
    }
}

/// Get the id that Robinhood uses to track a symbol.
///
/// This id is needed for some of the API endpoints. If you don't need a full
/// `StockInfo` struct, this is lighter weight than populating that.
/// e.g. For `AMD`, the id is `940fc3f5-1db5-4fed-b452-f3a2e4562b5f`.
fn get_robinhood_id(client: &mut Client,
                    symbol: &str)
    -> Result<String, Box<error::Error>>
{
    // We'll scrape the id from the "instrument" URL in this response.
    // If the API changes, this will start to fail.
    let info = robinhood::Fundamentals::load(client, symbol)?;
    // The path segments are expected to look like this:
    //      [ "instruments", "940fc3f5-1db5-4fed-b452-f3a2e4562b5f", ""]
    // If we don't get back a valid URL, or they don't look like this,
    // there's not much we can do.
    if let Some(mut segments) = Url::parse(&info.instrument)?
                                    .path_segments() {
        if let Some(id) = segments.nth(1) {
            return Ok(id.to_string());
        } else {
            eprintln!("ERROR: Malformed URL - Not enough path segments. Did Robinhood change their API?");
        }
    } else {
        eprintln!("ERROR: Malformed URL - This isn't a valid URL. How did you get here?");
    }
    Ok("ffffffff-ffff-ffff-ffff-ffffffffffff".to_string())
}

fn main() {
    match main2() {
        Ok(()) => {},
        Err(err) => eprintln!("{:#?}", err),
    }
}

fn main2() ->  Result<(), Box<error::Error>> {
    let mut client = Client::new();

    let bot = DiscordBot::new()?;
    bot.write("✓ - Hello!")?;

    let amd_id = get_robinhood_id(&mut client, "AMD")?;
    println!("AMD's Robinhood's Id = `{}`", amd_id);

    let amd_funda = robinhood::Fundamentals::load(&mut client, "AMD")?;
    bot.writef(format_args!("AMD Fundamentals:\n```\n{:#?}\n```", amd_funda))?;

    Ok(())
}
