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

use json::JsonValue;

use serenity::{
    http,
    model::webhook::Webhook,
    model::channel::Message,
};

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

/// Helper method to perform a GET request and parse the result into Json.
pub fn get_json(client: &mut reqwest::Client,
                url:    Url)
    -> Result<JsonValue, Box<error::Error>>
{
    let req = client.get(url).build()?;
    // This should always be a GET request.
    println!("{} \"{}\"", req.method(), req.url());

    let mut res = client.execute(req)?;
    if res.status() == reqwest::StatusCode::Ok {
        Ok(json::parse(&res.text()?)?)
    } else {
        eprintln!("Request failed with status code: {:?}", res.status());
        panic!("Couldn't make a reqwest Error object - API doesn't allow it.");
    }
}

fn get_everything(client: &mut reqwest::Client, symbol: &str)
    -> Result<JsonValue, Box<error::Error>>
{
    // This is our final Json object returned. It will contain *everything* we
    // can find on this symbol.
    // It's used so much, it helps to have a short name like "j", for Json.
    let mut j = json::JsonValue::new_object();

    // The "first" end-point is "Fundamentals", and is looked up by symbol.
    let url = format!("https://api.robinhood.com/fundamentals/{}/", symbol);
    j["fundamentals"] = get_json(client, Url::parse(&url)?)?;
    j["fundamentals"].remove("description"); // Too long to care.

    // The next endpoint that we check is "Instrument".
    // This is a natural follow-up, since we get the direct URL from
    // Fundamentals. This is also valuable for the "id" field, which gets us
    // the internal Robinhood Id, without having to parse a URL.
    let instr = &j["fundamentals"]["instrument"];
    let url: &str = instr.as_str().unwrap_or_default();
    j["instruments"] = get_json(client, Url::parse(&url)?)?;
    j["instruments"].remove("description"); // Too long to care.

    let id: String = j["instruments"]["id"].as_str()
                                           .unwrap_or_default().to_string();

    // "Quote" is information about the price of stock
    let instr = &j["instruments"]["quote"];
    let url: &str = instr.as_str().unwrap_or_default();
    j["quote"] = get_json(client, Url::parse(&url)?)?;

    // "Market" is information about the stock exchange in question.
    // e.g. It's NASDAQ for AMD.
    let instr = &j["instruments"]["market"];
    let url: &str = instr.as_str().unwrap_or_default();
    j["market"] = get_json(client, Url::parse(&url)?)?;

    Ok(j)
}

fn main() {
    match main2() {
        Ok(()) => {},
        Err(err) => eprintln!("{:#?}", err),
    }
}

fn main2() ->  Result<(), Box<error::Error>> {
    let bot = DiscordBot::new()?;
    bot.write("✓ - Hello!")?;

    let mut client = Client::new();
    let amd_json = get_everything(&mut client, "AMD")?;

    // Dump a human-readable version of the json.
    let pretty_utf8: &str;
    let mut pretty_buffer: Vec<u8> = vec![];
    amd_json.write_pretty(&mut pretty_buffer, 4 /*spaces*/)?;
    pretty_utf8 = str::from_utf8(&pretty_buffer)?;

    // And post it to chat - ignoring errors (probably about length)
    bot.writef(format_args!("AMD Json:\n```\n{}\n```\n", pretty_utf8));

    // Save interesting values
    // Print interesting changes

    Ok(())
}
