/* -*- mode: rust; indent-tabs-mode: nil; tab-width: 2 -*-
 *
 * main.rs - Main dns-over-https-proxy entry point.
 *
 * This file is part of dns-over-https-proxy:
 * https://github.com/micolous/dns-over-https-proxy
 *
 * Copyright 2017-2018 Michael Farrell <micolous+git@gmail.com>
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
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

