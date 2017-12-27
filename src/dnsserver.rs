use std::net::{UdpSocket, ToSocketAddrs};
use std::str::FromStr;

use domain::bits::message::Message;
use domain::iana::{Rcode, Rtype};
use domain::rdata::{A, Aaaa, Cname, Mx, Txt};
use domain::bits::{ComposeMode, DNameBuf, MessageBuilder};
use ::pdns::Pdns;
use std::error;


static DEFAULT_TTL : u32 = 180;

macro_rules! dns_error {
  ($socket:expr, $response:expr, $error_code:expr) => {{
    {
      let rheader = $response.header_mut();
      rheader.set_rcode($error_code);
    }
    
    Ok($response.finish().as_slice().to_vec())
  }};
}

pub struct DnsServer {
  socket: UdpSocket,
  pdns: Pdns,
}

impl DnsServer {
  pub fn new<A: ToSocketAddrs>(bind_addr: A) -> DnsServer {
    DnsServer {
      socket: UdpSocket::bind(bind_addr).expect("couldn't bind to addr"),
      pdns: Pdns::new(),
    }
  }
  
  pub fn handle_one_query(&mut self, buf: Vec<u8>) -> Result<Vec<u8>, Box<error::Error>> {
    let packet = Message::from_bytes(buf.as_slice())?;
    
    // Make sure we actually got a query, otherwise ignore it.
    if packet.header().qr() {
      warn!("Not a query, ignoring");
      return Ok(Vec::new());
    }

    let mut response = MessageBuilder::new(ComposeMode::Limited(1400), true)?;
    {
      let rheader = response.header_mut();
      rheader.set_id(packet.header().id());
      rheader.set_qr(true);
    }

    let question = match packet.first_question() {
      Some(question) => (question),
      None => {
        warn!("No question found in query");
        return dns_error!(self.socket, response, Rcode::FormErr);
      }
    };

    let hostname = format!("{}", question.qname().clone());
    let qtype = question.qtype().to_int();
    debug!("hostname: {}, type: {}", hostname, qtype);

    response.push(question)?;

    // Make a query
    let res = match self.pdns.lookup_hostname(hostname, qtype) {
      Ok(res) => (res),
      Err(e) => {
        warn!("Got error from DNS over HTTP: {}", e);
        return dns_error!(self.socket, response, Rcode::ServFail);
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
                DNameBuf::from_str(answer.name.as_str())?,
                answer.ttl.unwrap_or(DEFAULT_TTL),
                A::new(answer.data.parse()?)))?;
            },
            Rtype::Cname => {
              response.push((
                DNameBuf::from_str(answer.name.as_str())?,
                answer.ttl.unwrap_or(DEFAULT_TTL),
                Cname::new(DNameBuf::from_str(answer.data.as_str())?)))?;
            },
            Rtype::Mx => {
              let v: Vec<&str> = answer.data.as_str().split(' ').collect();
              response.push((
                DNameBuf::from_str(answer.name.as_str())?,
                answer.ttl.unwrap_or(DEFAULT_TTL),
                Mx::new(u16::from_str(v[0])?, DNameBuf::from_str(v[1])?)))?;
            }
            Rtype::Aaaa => {
              response.push((
                DNameBuf::from_str(answer.name.as_str())?,
                answer.ttl.unwrap_or(DEFAULT_TTL),
                Aaaa::new(answer.data.parse()?)))?;
            },
            Rtype::Txt => {
              // domain doesn't handle TXT records nicely.
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
                DNameBuf::from_str(answer.name.as_str())?,
                answer.ttl.unwrap_or(DEFAULT_TTL),
                Txt::new(o)))?;
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
    
    return Ok(response.finish().as_slice().to_vec());
  }
  
  pub fn run(&mut self) {
    loop {
      let mut buf = [0; 1400];
      
      let (size, src) = match self.socket.recv_from(&mut buf) {
        Ok((size, src)) => (size, src),
        Err(e) => {
          warn!("Error in recv: {}", e);
          continue;
        }
      };
      
      // Redeclare buf as the correct size
      let buf = &mut buf[..size];
      
      match self.handle_one_query(buf.to_vec()) {
        Ok(response) => {
          if response.len() > 0 {
            match self.socket.send_to(response.as_slice(), src) {
              Ok(n) => debug!("Data sent: {}", n),
              Err(e) => warn!("Error sending response: {}", e),
            }
          } else {
            debug!("Empty response");
          }
        },
        Err(e) => warn!("Error in handle_one_query: {}", e),
      };
    }
  }
}