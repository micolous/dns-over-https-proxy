use std::error;
use reqwest::{Client, Url};
use reqwest::header::UserAgent;
use rand::{Rng, OsRng};

#[derive(Deserialize, Debug)]
pub struct DnsQuestion {
  pub name: String,
  #[serde(rename="type")]
  pub typ: u8,
}

#[derive(Deserialize, Debug)]
pub struct DnsAnswer {
  pub name: String,
  #[serde(rename="type")]
  pub typ: u8,
  pub ttl: Option<u32>,
  pub data: String,
}

#[derive(Deserialize, Debug)]
pub struct DnsResponse {
  #[serde(rename="Status")]
  pub status: i32,
  #[serde(rename="TC")]
  pub truncated: bool,
  
  // "Always true for Google Public DNS"
  #[serde(rename="RD")]
  pub recursion_desired: bool,
  #[serde(rename="RA")]
  pub recursion_available: bool,
  
  #[serde(rename="AD")]
  pub dnssec_validated: bool,
  #[serde(rename="CD")]
  pub dnssec_disabled: bool,
  
  #[serde(rename="Question")]
  pub question: Vec<DnsQuestion>,
  
  #[serde(rename="Answer")]
  pub answer: Option<Vec<DnsAnswer>>,
  
  #[serde(rename="Comment")]
  pub comment: Option<String>,
}

static API_PATH: &'static str = "https://dns.google.com/resolve";

pub struct Pdns {
  rng: OsRng,
  client: Client,
}

// https://developers.google.com/speed/public-dns/docs/dns-over-https
impl Pdns {
  pub fn new() -> Pdns {
    Pdns {
      rng: OsRng::new().unwrap(),
      client: Client::new(),
    }
  }

  pub fn lookup_hostname(&mut self, hostname: String, qtype: u16) -> Result<DnsResponse, Box<error::Error>> {
    let random_padding_len = (self.rng.next_u32() & 0xf) as usize;
    let random_string = self.rng.gen_ascii_chars().take(random_padding_len).collect();

    let url = Url::parse_with_params(API_PATH, &[
      ("name", hostname),
      ("type", qtype.to_string()),
      ("random_padding", random_string)])?;

    debug!("url: {:?}", url);

    let mut response = self.client
      .get(url)
      .header(UserAgent::new("DnsOverHttpsProxy/1"))
      .send()?;

    let out: DnsResponse = response.json()?;

    Ok(out)
  }
}
