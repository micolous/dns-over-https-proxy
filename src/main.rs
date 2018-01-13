/* -*- mode: rust; indent-tabs-mode: nil; tab-width: 2 -*- */
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

use std::env;
use ::dnsserver::DnsServer;

fn main() {
  env_logger::init().unwrap();
  let args: Vec<_> = env::args().collect();

  let mut bind_addr = "127.0.0.1:35353";
  if args.len() > 1 {
		bind_addr = &args[1];
  }

  let mut dns_server = DnsServer::new(bind_addr);
  dns_server.run();
}

