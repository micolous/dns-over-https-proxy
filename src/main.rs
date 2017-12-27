#[macro_use] extern crate log;
extern crate env_logger;
extern crate reqwest;
extern crate rand;
extern crate domain;
extern crate serde;
extern crate serde_json;
#[macro_use] extern crate serde_derive;

pub mod pdns;
pub mod dnsserver;

use ::dnsserver::DnsServer;

fn main() {
  env_logger::init().unwrap();
  let mut dns_server = DnsServer::new("127.0.0.1:35353");
  dns_server.run();


}



