use std::{
    env,
    error,
    fmt,
    ops,
    str,
};

use curl::easy;
use json;
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
                eprintln!("WARNING: Message contains \"```\" - it will not be sent shortened!");
            }
        }
        Ok(self.webhook.execute(/*wait?*/ false, |w| w.content(msg))?)
    }

    pub fn writef(&self, args: fmt::Arguments) -> Result<Option<Message>, Box<error::Error>> {
        let utf8 = format!("{}", args);
        self.write(&utf8)
    }
}

struct Collector(Vec<u8>);

impl ops::Deref for Collector {
    type Target = Vec<u8>;
    fn deref(&self) -> &Vec<u8> { &self.0 }
}

impl ops::DerefMut for Collector {
    fn deref_mut(&mut self) -> &mut Vec<u8> { &mut self.0 }
}

impl easy::Handler for Collector {
    fn write(&mut self, data: &[u8]) -> Result<usize, easy::WriteError> {
        self.0.extend_from_slice(data);
        Ok(data.len())
    }
}

// TODO: Make this a configurable list of names.
struct StockInfo {
    // Fields are returned as Strings, so we'll save them as Strings.
    // They're just going to be made human-readable anyway.

    rh_id:                  String, // e.g. "940fc3f5-1db5-4fed-b452-f3a2e4562b5f"
    bloomberg_unique:       String, // e.g. "EQ0010001000001000"
    updated_at:             String, // Date

    previous_close_date:    String, // Date
    previous_close:         String, // Number

    open:                   String, // Number
    high:                   String, // Number
    low:                    String, // Number

    shares_outstanding:     String, // Integer
    volume:                 String, // Integer
    average_volume_2_weeks: String, // Number/Integer
    average_volume:         String, // Number/Integer

    pe_ratio:               String, // Number
    num_employees:          String, // Integer
}

/// Get the id that Robinhood uses to track a symbol.
///
/// This id is needed for some of the API endpoints. If you don't need a full
/// `StockInfo` struct, this is lighter weight than populating that.
/// e.g. For `AMD`, the id is `940fc3f5-1db5-4fed-b452-f3a2e4562b5f`.
fn get_robinhood_id(handle: &mut easy::Easy2<Collector>, stock: &str) -> Result<String, Box<error::Error>> {
    // TODO: Combine URLs correctly?
    let url = format!("https://api.robinhood.com/fundamentals/{}/", stock);

    let stock_fundamentals: json::JsonValue;

    handle.get(true)?;
    handle.get_mut().clear();
    handle.url(&url)?;
    if 1 == 1 {
        let mut headers = easy::List::new();
        headers.append("Accept: application/json")?;
        handle.http_headers(headers)?;
    }
    handle.perform()?;

    let response_code = handle.response_code()?;
    if response_code != 200 {
        panic!("HTTP Response Code: {}", response_code);
    }
    let contents = handle.get_ref();
    let utf8 = String::from_utf8_lossy(&contents);
    stock_fundamentals = json::parse(&utf8)?;

    if let Some(instrument_url) = stock_fundamentals["instrument"].as_str() {
        // Here's what instrument_url might look like:
        //      https://api.robinhood.com/instruments/940fc3f5-1db5-4fed-b452-f3a2e4562b5f/
        // We'll split the URL on '/', and then examine the last non-empty str.
        let mut iter = instrument_url
            .split('/')
            // Skip all empty strings, e.g. from a trailing '/' or "http://"
            .filter(|s| s.len() > 0);
        if let Some(id) = iter.next_back() {
            return Ok(id.to_string());
        }
    }
    // Something went wrong.
    // TODO: Find out what.
    panic!("??");
}

fn get_stock_info(handle: &mut easy::Easy2<Collector>,
                  stock:  &str)
    -> Result<json::JsonValue, Box<error::Error>>
{
    let stock_info = json::JsonValue::new_object();

    // Query Robinhood's "Fundamentals" endpoint.
    handle.get_mut().clear();
    let stock_fundamentals: json::JsonValue;
    let instrument_url: &str;
    {
        // TODO: Combine URLs correctly?
        let url = format!("https://api.robinhood.com/fundamentals/{}/", stock);
        // handle.get(true)?;
        handle.url(&url)?;
        handle.perform()?;

        if handle.response_code()? != 200 {
            panic!("HTTP Response Code: {}", handle.response_code()?);
        }
        let contents = handle.get_ref();
        let utf8 = String::from_utf8_lossy(&contents);
        stock_fundamentals = json::parse(&utf8)?;
        instrument_url = stock_fundamentals["instrument"].as_str().unwrap_or_default();
    }

    // Query "Instruments"
    handle.get_mut().clear();
    let stock_fundamentals: json::JsonValue;
    {
        handle.url(instrument_url)?;
        let mut headers = easy::List::new();
        handle.perform()?;

        if handle.response_code()? != 200 {
            panic!("HTTP Response Code: {}", handle.response_code()?);
        }
        let contents = handle.get_ref();
        let utf8 = String::from_utf8_lossy(&contents);
        stock_fundamentals = json::parse(&utf8)?;
    }

    Ok(stock_fundamentals)
}

fn main() {
    match main2() {
        Ok(()) => {},
        Err(err) => eprintln!("{:#?}", err),
    }
}

fn main2() ->  Result<(), Box<error::Error>> {
    let mut handle = easy::Easy2::new(Collector(vec![]));

    let bot = DiscordBot::new()?;
    bot.write("`DiscordBot::new()` ✓")?;

    let amd_id = get_robinhood_id(&mut handle, "AMD")?;
    bot.writef(format_args!("AMD's Robinhood's Id = `{}`", amd_id))?;

    let amd_pretty;
    let mut amd_pretty_buffer: Vec<u8> = vec![];
    let amd_json = get_stock_info(&mut handle, "AMD")?;

    amd_json.write_pretty(&mut amd_pretty_buffer, /*spaces*/ 4)?;
    amd_pretty = String::from_utf8_lossy(&amd_pretty_buffer);
    bot.writef(format_args!("```\n{}\n```\n", &amd_pretty))?;

    Ok(())
}
