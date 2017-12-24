#[macro_use] extern crate log;
extern crate env_logger;
extern crate reqwest;
extern crate rand;
extern crate domain;
extern crate serde;
extern crate serde_json;
#[macro_use] extern crate serde_derive;

pub mod pdns;

use std::net::UdpSocket;
use std::str::FromStr;

use domain::bits::message::Message;
use domain::iana::{Rcode};
use domain::rdata::{A, Aaaa, Cname, Mx};
use domain::bits::{ComposeMode, DNameBuf, MessageBuilder};
use ::pdns::Pdns;

static DEFAULT_TTL : u32 = 180;

fn main() {
  env_logger::init().unwrap();
  let mut pdns = Pdns::new();

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
    response.push(question).unwrap();

    // Make a query
    let res = match pdns.lookup_hostname(hostname, qtype) {
      Ok(res) => (res),
      Err(e) => {
        warn!("Got error from DNS over HTTP: {}", e);
        
        {
          let rheader = response.header_mut();
          rheader.set_rcode(Rcode::ServFail);
        }

        match socket.send_to(response.finish().as_slice(), src) {
          Ok(n) => debug!("Data sent: {}", n),
          Err(e) => warn!("Error sending response: {}", e),      
        }        
        continue;
      }
    };

    debug!("Response: {:?}", res);

    {
      let rheader = response.header_mut();    
      rheader.set_rd(res.recursion_desired);
      rheader.set_ra(res.recursion_available);
      rheader.set_ad(res.dnssec_validated);
      rheader.set_cd(res.dnssec_disabled);
    }    

    
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
        let rheader = response.header_mut();
        rheader.set_rcode(Rcode::NXDomain);
      } 
    }
    
    match socket.send_to(response.finish().as_slice(), src) {
      Ok(n) => debug!("Data sent: {}", n),
      Err(e) => warn!("Error sending response: {}", e),      
    }
  }
}



