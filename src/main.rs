extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;

extern crate reqwest;
extern crate rand;

extern crate hex_slice;
extern crate dns_parser;

use hex_slice::AsHex;

use reqwest::Client;
use reqwest::Url;
use reqwest::header::UserAgent;
use std::io;
use std::error;

use std::net::UdpSocket;

use std::result::Result;
use rand::Rng;
use rand::OsRng;
use dns_parser::Packet;

#[derive(Deserialize, Debug)]
struct DnsQuestion {
  name: String,
  #[serde(rename="type")]
  typ: i32,
}

#[derive(Deserialize, Debug)]
struct DnsAnswer {
  name: String,
  #[serde(rename="type")]
  typ: i32,
  ttl: Option<i32>,
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

// https://developers.google.com/speed/public-dns/docs/dns-over-https

fn lookup_hostname(rng: &mut OsRng, hostname: String, record_type: String) -> Result<DnsResponse, Box<error::Error>> {
  
  let random_padding_len = (rng.next_u32() & 0xf) as usize;
  let random_string = rng.gen_ascii_chars().take(random_padding_len).collect();
  
  println!("random_string = {}", random_string);
  
  let url = Url::parse_with_params(API_PATH, &[
    ("name", hostname),
    ("type", record_type),
    ("random_padding", random_string)])?;
    
  let mut response = Client::new()
    .get(url)
    .header(UserAgent::new("DnsOverHttpsProxy/1"))
    .send()?;
  
  let out: DnsResponse = response.json()?;
  
  Ok(out)
}

fn main() {
  let mut rng = rand::os::OsRng::new().unwrap();
  
  let socket = UdpSocket::bind("127.0.0.1:35353").expect("couldn't bind to addr");
  let mut buf = [0; 1440];
  
  loop {
    let (size, src) = match socket.recv_from(&mut buf) {
      Ok((size, src)) => (size, src),
      Err(e) => {
        println!("Error in recv: {}", e);
        continue;
      }
    };
    
    // Redeclare buf as the correct size
    let mut buf = &mut buf[..size];
  
    let packet = match Packet::parse(&mut buf) {
      Ok(packet) => (packet),
      Err(e) => {
        println!("Error parsing DNS packet: {}", e);
        continue;
      }
    };
    
    println!("Packet: {:?}", packet);
  }
  /*
  
  println!("Enter the hostname to resolve");
  let mut hostname = String::new();
  
  io::stdin().read_line(&mut hostname)
    .expect("Couldn't read line");
  
  // Strip whitespace characters from the string
  hostname = String::from(hostname.trim());
  
  let rtype = String::from("A");
  
  match lookup_hostname(&mut rng, hostname, rtype) {
    Ok(res) => println!("Got response: {:?}", res),
    Err(e) => println!("Got error: {}", e),
  };
  */
}



