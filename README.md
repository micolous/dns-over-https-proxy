# dns-over-https-proxy

This is a simple DNS server that proxies requests to [Google Public DNS](https://developers.google.com/speed/public-dns/docs/dns-over-https).

It is not yet complete.  It currently supports a limited number of record types.

It is intended for use on either a single machine, or behind a better (ideally caching) DNS server like [BIND](https://www.isc.org/downloads/bind/), in order to reduce clear-text DNS being transmitted over the internet.  This relies on the premise that you [trust Google with all of your DNS traffic](https://developers.google.com/speed/public-dns/privacy).

> **Note**: This is not an official Google product. Please don't bug them about this. :)

## Building it

[Install the current version of Rust](https://www.rustup.rs) and dependencies:

```
sudo apt install build-essential curl libssl-dev pkgconfig
curl https://sh.rustup.rs -sSf | sh
. ~/.cargo/env
```

Then:

```
cargo build --release
```

You'll get a binary in `target/release/dns-over-https-proxy`.

## Running it

By default, it listens on `127.0.0.1:35353` (UDP). This can be changed by specifying a different host/port as a command-line option, for example:

```
dns-over-https-proxy 0.0.0.0:53
```

IPv6 is also supported:

```
dns-over-https-proxy '[::1]:35353'
```

Debug mode (environment variable): `RUST_LOG=dns_over_https_proxy=debug`.  Note that this debug mode will dump all received DNS queries and responses to stderr.

## Running with systemd

There is a unit file included with this repository, in `dns-over-https-proxy.service`.  Once the binary is built, you can set things up (as root):

```
install -o0 -g0 target/release/dns-over-https-proxy /usr/local/sbin/
install -m644 -o0 -g0 dns-over-https-proxy.service /etc/systemd/system/
systemctl daemon-reload
systemctl enable dns-over-https-proxy.service
systemctl start dns-over-https-proxy.service
```

There also exists an alternative target which can be used to start up multiple instances of the program on different ports, in order to allow a DNS server to load balance between processes (`dns-over-https-proxy@.service`):

```
install -o0 -g0 target/release/dns-over-https-proxy /usr/local/sbin/
install -m644 -o0 -g0 dns-over-https-proxy@.service /etc/systemd/system/
systemctl daemon-reload

# Start instances on 127.0.0.1 port 1230, 1231, 1232 & 1233
for p in 1230 1231 1232 1233; do
	systemctl enable dns-over-https-proxy@127.0.0.1\:$p
	systemctl start dns-over-https-proxy@127.0.0.1\:$p
done
```

## Running in Docker

Many distributions don't ship with new enough versions of Rust, and it is very fast-moving.  For convenience, a Docker container is offered.

```
docker build -t dns-over-https-proxy .
docker run -d -p 127.0.0.1:35353:35353/udp --rm --name dns-over-https-proxy dns-over-https-proxy
```

This will build and start a Docker container, with DNS available on `localhost:35353`.

## Using the DNS server with dig

```
dig -p 35353 developers.google.com @127.0.0.1
```

## Using with BIND

The primary goal of this project is to act as a forwarder for BIND. However, there is a Catch-22 on the DNS server:

* This program connects to `dns.google.com`, and needs to know the IP address of it.

* The system resolver contacts BIND to look up the name.

* BIND needs an upstream resolver in order to resolve `dns.google.com`, and contacts this program.

* ...and repeat.

### Breaking the loop with BIND

BIND has configuration options that allow us to break the loop, at the expense of leaking DNS queries to `dns.google.com`. While an attacker (or your ISP) could intercept your requests to 8.8.8.8, `reqwests` (when used with a functional OpenSSL implementation, and reliable certificate chain) will fail in the event that your traffic has been redirected.

This configuration uses Google Public DNS via the DNS protocol (unencrypted), and then forwards everything else to this program.

```
// In local view / global zone config

zone "dns.google.com." {
	type forward;
	forwarders {
		8.8.8.8;
		8.8.4.4;
		2001:4860:4860::8888;
		2001:4860:4860::8844;
	};
};

// In named.conf.options
options {
	forwarders {
		127.0.0.1 port 35353;

		// Additional forwarders can be added here.
	};

	// The default behaviour will leak DNS queries to upstream servers in the
	// event of errors. This will return SERVFAIL on the proxy being unavailable.
	forward only;
};
```

### Breaking the loop with the system resolver

The system resolver can also break this loop in two different ways:

* Use a DNS server other than `localhost`.

* Hard code IP addresses for `dns.google.com` in `/etc/hosts`, eg:

```
216.239.32.27 dns.google.com
```

## Supported record types

Per [RFC 1035](https://www.ietf.org/rfc/rfc1035.txt) unless otherwise stated:

* A
* AAAA (RFC 3596)
* CNAME
* MX
* NS
* PTR
* SOA
* SRV (RFC 2782)
* TXT

