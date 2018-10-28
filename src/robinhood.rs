
use std::error;

use serde_derive::{
    Serialize,
    Deserialize
};

#[derive(Clone, Debug)]
#[derive(Serialize, Deserialize)]
pub struct Fundamentals {
    pub open:                   String, // e.g. "18.490000",
    pub high:                   String, // e.g. "18.770000",
    pub low:                    String, // e.g. "17.250000",
    pub volume:                 String, // e.g. "56573765.000000",
    pub average_volume_2_weeks: String, // e.g. "112203294.200000",
    pub average_volume:         String, // e.g. "75154888.127500",
    pub high_52_weeks:          String, // e.g. "34.140000",
    pub dividend_yield:         String, // e.g. "0.000000",
    pub low_52_weeks:           String, // e.g. "9.040000",
    pub market_cap:             String, // e.g. "17189250000.000000",
    pub pe_ratio:               String, // e.g. "51.131090",
    pub shares_outstanding:     String, // e.g. "975000000.000000",
    pub description:            String, // A short paragraph of text about the company.
    pub instrument:             String, // e.g. "https://api.robinhood.com/instruments/940fc3f5-1db5-4fed-b452-f3a2e4562b5f/",
    pub ceo:                    String, // e.g. "Lisa T. Su",
    pub headquarters_city:      String, // e.g. "Santa Clara",
    pub headquarters_state:     String, // e.g. "California",
    pub sector:                 String, // e.g. "Electronic Technology",
    pub industry:               String, // e.g. "Semiconductors",
    pub num_employees:          u64,    // e.g. 8900,
    pub year_founded:           u64,    // e.g. 1969
}

impl Fundamentals {
    pub fn load(client: &mut reqwest::Client,
                symbol: &str)
    -> Result<Fundamentals, Box<error::Error>>
    {
        let url = reqwest::Url::parse(
                &format!("https://api.robinhood.com/fundamentals/{}/",
                         symbol))?;

        let req = client.get(url).build()?;
        println!("{} \"{}\"", req.method(), req.url());

        let mut res = client.execute(req)?;
        if res.status() == reqwest::StatusCode::Ok {
            Ok(res.json()?)
        } else {
            eprintln!("Request failed with status code: {:?}", res.status());
            panic!("Couldn't make a reqwest Error object - API doesn't allow it.");
        }
    }
}
