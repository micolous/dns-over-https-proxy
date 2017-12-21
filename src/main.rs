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
use domain::bits::record::Record;
use domain::iana::class::Class;
use domain::rdata::A;
use domain::bits::{ComposeMode, DNameBuf, MessageBuilder, Question};


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
  typ: u8,
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

fn lookup_hostname(rng: &mut OsRng, hostname: String, qtype: u16) -> Result<DnsResponse, Box<error::Error>> {
  
  let random_padding_len = (rng.next_u32() & 0xf) as usize;
  let random_string = rng.gen_ascii_chars().take(random_padding_len).collect();
  
  let url = Url::parse_with_params(API_PATH, &[
    ("name", hostname),
    ("type", qtype.to_string()),
    ("random_padding", random_string)])?;

  println!("url: {:?}", url);

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
  let mut buf = [0; 1400];
  
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
  
    let packet = match Message::from_bytes(&mut buf) {
      Ok(packet) => (packet),
      Err(e) => {
        println!("Error parsing DNS packet: {}", e);
        continue;
      }
    };
    
    // Make sure we actually got a query, otherwise ignore it.
    if packet.header().qr() {
      println!("Not a query, ignoring");
      continue;
    }
    
    let question = match packet.first_question() {
      Some(question) => (question),
      None => {
        println!("No question found in query, ignoring");
        continue;
      }
    };
    
    let hostname = format!("{}", question.qname().clone());
    let qtype = question.qtype().to_int();
    println!("hostname: {}, type: {}", hostname, qtype);
    
    // Make a query
    let res = match lookup_hostname(&mut rng, hostname, qtype) {
      Ok(res) => (res),
      Err(e) => {
        println!("Got error from DNS over HTTP: {}", e);
        continue;
      }
    };
    
    println!("Response: {:?}", res);
    
    let mut response = MessageBuilder::new(ComposeMode::Limited(1400), true).unwrap();
    let mut rheader = response.header_mut();
    rheader.set_id(packet.header().id());
    rheader.set_qr(true);
    response.push(question);
    
    let mut response = response.answer();
    match res.answer {
      Some(answers) => {
        for answer in answers {
          match answer.typ {
            1 => {
              response.push((DNameBuf::from_str(answer.name.as_str()).unwrap(), answer.ttl, A::new(answer.data.parse().unwrap()))).unwrap();
            }
            _ => {
              println!("unhandled response type {}", answer.typ);
            }
          }
        }
      },
      None => {
        println!("todo: handle null response");
      } 
    }
    
    match socket.send_to(response.finish().as_slice(), src) {
      Ok(n) => println!("Data sent: {}", n),
      Err(e) => println!("Error sending response: {}", e),      
    }
    
    // Craft response
    //let mut response =
    //packet.header.query = false;
    //packet.header.answers = 
    
    // Send our response
    /*match socket.send_to(..., src) {
      Ok(n) => println!("Data sent: {}", n),
      Err(e) => println!("Error sending response: {}", e),
    }*/
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



