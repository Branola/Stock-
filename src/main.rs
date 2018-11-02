use std::{
    env,
    error,
    fmt,
    str,
    thread,
    time,
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

// Error handling the way God intended.
type Result<T> = std::result::Result<T, Box<error::Error>>;

// The maximum number of characters allowed in a Discord message.
// Updated: October 28, 2018
const DISCORD_MAX_MSG_CHAR_LEN: usize = 2000;

#[derive(Debug)]
struct PersistantState {
    symbol: String,
    bot:    DiscordBot,
    client: Client,
    calls:  u64,

    // The last known price, in cents.
    last_price: u64,
}

impl PersistantState {
    fn new() -> Result<PersistantState> {
        Ok(PersistantState {
            symbol: env::var("STOCK_SYMBOL").expect("set STOCK_SYMBOL"),
            bot:    DiscordBot::new()?,
            client: Client::new(),
            calls:  0,
            last_price: 0,
        })
    }

    // Returns "self.last_price", as a float in USD, e.g. 2.00.
    fn last_price_usd(&self) -> f64 {
        self.last_price as f64 / 100.0
    }
}

#[derive(Debug)]
struct DiscordBot {
    webhook: Webhook,
}

impl DiscordBot {
    pub fn new() -> Result<DiscordBot> {
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
    pub fn write(&self, msg: &str) -> Result<Option<Message>> {

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
    pub fn writef(&self, args: fmt::Arguments) -> Result<Option<Message>> {
        let utf8 = format!("{}", args);
        self.write(&utf8)
    }
}

/// Helper method to perform a GET request and parse the result into Json.
pub fn get_json(client: &mut reqwest::Client, url: Url) -> Result<JsonValue> {
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
    -> Result<JsonValue>
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

    // "Quote" is information about the price of stock
    let quote = &j["instruments"]["quote"];
    let url: &str = quote.as_str().unwrap_or_default();
    j["quote"] = get_json(client, Url::parse(&url)?)?;
    j["quote"].remove("instrument"); // We already have this URL.

    // "Market" is information about the stock exchange in question.
    // e.g. It's NASDAQ for AMD.
    let market = &j["instruments"]["market"];
    let url: &str = market.as_str().unwrap_or_default();
    j["market"] = get_json(client, Url::parse(&url)?)?;

    Ok(j)
}

fn query_stock(state: &mut PersistantState) -> Result<()> {
    let amd_json = get_everything(&mut state.client, &state.symbol)?;

    state.calls += 1;
    let price_usd = amd_json["quote"]["last_trade_price"]
        .as_str().ok_or("\"quote/last_trade_price\" not found")?
        .parse::<f64>()?;

    let last_price_usd = state.last_price_usd();
    state.last_price = (price_usd * 100.0).round() as u64;

    #[derive(Debug)]
    struct StatusUpdate {
        total_calls: u64,
        last_price_usd: f64,
    }
    println!("{:?}", StatusUpdate {
        total_calls: state.calls,
        last_price_usd: state.last_price_usd(),
    });

    // Dump a human-readable version of the json.
    let mut pretty_buffer: Vec<u8> = vec![];
    amd_json["quote"].write_pretty(&mut pretty_buffer, 4 /*spaces*/)?;
    let _pretty_utf8: &str = str::from_utf8(&pretty_buffer)?;
    // println!("{}", pretty_utf8);

    // We need to know whether we should alter the chat.
    // We only alert the chat if the price has crossed a dollar boundary.
    // e.g. 17.01 -> 17.99 => No alert
    //      17.99 -> 18.01 => Alert
    let diff = state.last_price_usd().round() as i64
               - last_price_usd       .round() as i64;
    if diff < 0 {
        // And post it to chat - ignoring errors (probably about length)
        let _ = state.bot.writef(format_args!(
                "AMD price is DOWN to ${:4.2}\n", price_usd));
    } else if diff > 0 {
        // And post it to chat - ignoring errors (probably about length)
        let _ = state.bot.writef(format_args!(
                "AMD price is UP to ${:4.2}\n", price_usd));
    } else {
        // No messages if there are no changes over a dollar line.
    }

    Ok(())
}

fn main() {
    match main2() {
        Ok(()) => {},
        Err(err) => eprintln!("{:#?}", err),
    }
}

fn main2() ->  Result<()> {
    let delay = time::Duration::from_secs(1*60);

    let mut state = PersistantState::new()?;
    state.bot.write("✓ - Hello!")?;
    println!("✓ - Hello!");

    loop {
        match query_stock(&mut state) {
            Ok(()) => {},
            Err(err) => {
                eprintln!("ERROR: {:?}", err);
                state.bot.write("✗ - Error in processing update.")?;
            }
        }
        println!("");
        thread::sleep(delay);
    }
}
