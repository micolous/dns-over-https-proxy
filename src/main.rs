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
use domain::iana::{Rcode, Rtype};
use domain::rdata::{A, Aaaa, Cname, Mx, Txt};
use domain::bits::{ComposeMode, DNameBuf, MessageBuilder};
use ::pdns::Pdns;

static DEFAULT_TTL : u32 = 180;

macro_rules! dns_error {
  ($socket:expr, $src:expr, $response:expr, $error_code:expr) => {{
    {
      let rheader = $response.header_mut();
      rheader.set_rcode($error_code);
    }

    match $socket.send_to($response.finish().as_slice(), $src) {
      Ok(n) => debug!("Data sent: {}", n),
      Err(e) => warn!("Error sending response: {}", e),
    }
  }};
}

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

    let mut response = MessageBuilder::new(ComposeMode::Limited(1400), true).unwrap();
    {
      let rheader = response.header_mut();
      rheader.set_id(packet.header().id());
      rheader.set_qr(true);
    }

    let question = match packet.first_question() {
      Some(question) => (question),
      None => {
        warn!("No question found in query");
        dns_error!(socket, src, response, Rcode::FormErr);
        continue;
      }
    };

    let hostname = format!("{}", question.qname().clone());
    let qtype = question.qtype().to_int();
    debug!("hostname: {}, type: {}", hostname, qtype);

    response.push(question).unwrap();

    // Make a query
    let res = match pdns.lookup_hostname(hostname, qtype) {
      Ok(res) => (res),
      Err(e) => {
        warn!("Got error from DNS over HTTP: {}", e);
        dns_error!(socket, src, response, Rcode::ServFail);
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
      rheader.set_rcode(Rcode::from_int(res.status));
    }


    let mut response = response.answer();
    match res.answer {
      Some(answers) => {
        for answer in answers {
          match Rtype::from_int(answer.typ) {
            Rtype::A => {
              response.push((
                DNameBuf::from_str(answer.name.as_str()).unwrap(),
                answer.ttl.unwrap_or(DEFAULT_TTL),
                A::new(answer.data.parse().unwrap()))).unwrap();
            },
            Rtype::Cname => {
              response.push((
                DNameBuf::from_str(answer.name.as_str()).unwrap(),
                answer.ttl.unwrap_or(DEFAULT_TTL),
                Cname::new(DNameBuf::from_str(answer.data.as_str()).unwrap()))).unwrap();
            },
            Rtype::Mx => {
              let v: Vec<&str> = answer.data.as_str().split(' ').collect();
              response.push((
                DNameBuf::from_str(answer.name.as_str()).unwrap(),
                answer.ttl.unwrap_or(DEFAULT_TTL),
                Mx::new(u16::from_str(v[0]).unwrap(), DNameBuf::from_str(v[1]).unwrap()))).unwrap();
            }
            Rtype::Aaaa => {
              response.push((
                DNameBuf::from_str(answer.name.as_str()).unwrap(),
                answer.ttl.unwrap_or(DEFAULT_TTL),
                Aaaa::new(answer.data.parse().unwrap()))).unwrap();
            },
            Rtype::Txt => {
              // domain doesn't handle TXT records properly.
              // Google Public DNS puts double quotes (") around the text
              // content of the record.
              let mut o: Vec<u8> = Vec::new();
              for t in answer.data.split("\"") {
                if t.len() == 0 {
                  continue;
                }

                o.push(t.len() as u8);
                o.extend_from_slice(t.as_bytes());
              }

              response.push((
                DNameBuf::from_str(answer.name.as_str()).unwrap(),
                answer.ttl.unwrap_or(DEFAULT_TTL),
                Txt::new(o))).unwrap();
            }
            // TODO: handle other things
            _ => {
              warn!("unhandled response type {}", answer.typ);
            }
          }
        }
      },
      None => {
        // Ignore empty response
      }
    }

    match socket.send_to(response.finish().as_slice(), src) {
      Ok(n) => debug!("Data sent: {}", n),
      Err(e) => warn!("Error sending response: {}", e),
    }
  }
}



