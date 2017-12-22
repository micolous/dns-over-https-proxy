#[macro_use] extern crate log;
extern crate env_logger;

extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;

extern crate reqwest;
extern crate rand;
extern crate domain;

use reqwest::Client;
use reqwest::Url;
use reqwest::header::UserAgent;
use std::error;

use std::net::UdpSocket;

use std::result::Result;
use rand::Rng;
use rand::OsRng;
use std::str::FromStr;

use domain::bits::message::Message;
//use domain::iana::{Class, Rtype};
use domain::rdata::{A, Aaaa, Cname, Mx};
use domain::bits::{ComposeMode, DNameBuf, MessageBuilder};

#[derive(Deserialize, Debug)]
struct DnsQuestion {
  name: String,
  #[serde(rename="type")]
  typ: u8,
}

#[derive(Deserialize, Debug)]
struct DnsAnswer {
  name: String,
  #[serde(rename="type")]
  typ: u8,
  ttl: Option<u32>,
  data: String,
}

#[derive(Deserialize, Debug)]
struct DnsResponse {
  #[serde(rename="Status")]
  status: i32,
  #[serde(rename="TC")]
  truncated: bool,
  
  // "Always true for Google Public DNS"
  //#[serde(rename="RD")]
  //rd: bool,
  //#[serde(rename="RA")]
  //ra: bool,
  
  #[serde(rename="AD")]
  dnssec_validated: bool,
  #[serde(rename="CD")]
  dnssec_disabled: bool,
  
  #[serde(rename="Question")]
  question: Vec<DnsQuestion>,
  
  #[serde(rename="Answer")]
  answer: Option<Vec<DnsAnswer>>,
  
  #[serde(rename="Comment")]
  comment: Option<String>,
}

static API_PATH: &'static str = "https://dns.google.com/resolve";
static DEFAULT_TTL : u32 = 180;

// https://developers.google.com/speed/public-dns/docs/dns-over-https

fn lookup_hostname(rng: &mut OsRng, hostname: String, qtype: u16) -> Result<DnsResponse, Box<error::Error>> {
  let random_padding_len = (rng.next_u32() & 0xf) as usize;
  let random_string = rng.gen_ascii_chars().take(random_padding_len).collect();

  let url = Url::parse_with_params(API_PATH, &[
    ("name", hostname),
    ("type", qtype.to_string()),
    ("random_padding", random_string)])?;

  debug!("url: {:?}", url);

  let mut response = Client::new()
    .get(url)
    .header(UserAgent::new("DnsOverHttpsProxy/1"))
    .send()?;

  let out: DnsResponse = response.json()?;

  Ok(out)
}

fn main() {
  env_logger::init().unwrap();
  let mut rng = rand::os::OsRng::new().unwrap();

  let socket = UdpSocket::bind("127.0.0.1:35353").expect("couldn't bind to addr");
  let mut buf = [0; 1400];

  info!("Listening for DNS requests on port 35353");
  
  loop {
    let (size, src) = match socket.recv_from(&mut buf) {
      Ok((size, src)) => (size, src),
      Err(e) => {
        warn!("Error in recv: {}", e);
        continue;
      }
    };
    
    // Redeclare buf as the correct size
    let mut buf = &mut buf[..size];
  
    let packet = match Message::from_bytes(&mut buf) {
      Ok(packet) => (packet),
      Err(e) => {
        warn!("Error parsing DNS packet: {}", e);
        continue;
      }
    };
    
    // Make sure we actually got a query, otherwise ignore it.
    if packet.header().qr() {
      warn!("Not a query, ignoring");
      continue;
    }
    
    let question = match packet.first_question() {
      Some(question) => (question),
      None => {
        warn!("No question found in query, ignoring");
        continue;
      }
    };
    
    let hostname = format!("{}", question.qname().clone());
    let qtype = question.qtype().to_int();
    debug!("hostname: {}, type: {}", hostname, qtype);
    
    let mut response = MessageBuilder::new(ComposeMode::Limited(1400), true).unwrap();
    {
      let rheader = response.header_mut();
      rheader.set_id(packet.header().id());
      rheader.set_qr(true);
    }

    // Make a query
    let res = match lookup_hostname(&mut rng, hostname, qtype) {
      Ok(res) => (res),
      Err(e) => {
        warn!("Got error from DNS over HTTP: {}", e);
        
        continue;
      }
    };

    debug!("Response: {:?}", res);
    
    response.push(question).unwrap();
    
    let mut response = response.answer();
    match res.answer {
      Some(answers) => {
        for answer in answers {
          match answer.typ {
            1 => { // A
              response.push((
                DNameBuf::from_str(answer.name.as_str()).unwrap(),
                answer.ttl.unwrap_or(DEFAULT_TTL),
                A::new(answer.data.parse().unwrap()))).unwrap();
            },
            5 => { // CNAME
              response.push((
                DNameBuf::from_str(answer.name.as_str()).unwrap(),
                answer.ttl.unwrap_or(DEFAULT_TTL),
                Cname::new(DNameBuf::from_str(answer.data.as_str()).unwrap()))).unwrap();
            },
            15 => { // MX
              let v: Vec<&str> = answer.data.as_str().split(' ').collect();
              response.push((
                DNameBuf::from_str(answer.name.as_str()).unwrap(),
                answer.ttl.unwrap_or(DEFAULT_TTL),
                Mx::new(u16::from_str(v[0]).unwrap(), DNameBuf::from_str(v[1]).unwrap()))).unwrap();              
            }
            28 => { // AAAA
              response.push((
                DNameBuf::from_str(answer.name.as_str()).unwrap(),
                answer.ttl.unwrap_or(DEFAULT_TTL),
                Aaaa::new(answer.data.parse().unwrap()))).unwrap();
            },
            // TODO: handle other things
            _ => {
              warn!("unhandled response type {}", answer.typ);
            }
          }
        }
      },
      None => {
        warn!("todo: handle null response");
      } 
    }
    
    match socket.send_to(response.finish().as_slice(), src) {
      Ok(n) => debug!("Data sent: {}", n),
      Err(e) => warn!("Error sending response: {}", e),      
    }
  }
}



