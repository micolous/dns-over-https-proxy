# dns-over-https-proxy

This is a simple DNS server that proxies requests to [Google Public DNS](https://developers.google.com/speed/public-dns/docs/dns-over-https).

It is not yet complete.  It currently supports `A`, `AAAA`, `CNAME`, `MX` and `TXT` records.

It is intended for use on either a single machine, or behind a better (ideally caching) DNS server like [BIND](https://www.isc.org/downloads/bind/).

## Building it

```
cargo build
```

## Running it

By default, it listens on `127.0.0.1:35353` (UDP). This can be changed by specifying a different host/port as a command-line option, for example:

```
dns-over-https-proxy 0.0.0.0:53
```

IPv6 is also supported:

```
dns-over-https-proxy '[::1]:35353'
```

Debug mode (environment variable): `RUST_LOG=dns-over-https-proxy=debug`

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
