# dns-over-https-proxy

This is a simple DNS server that proxies requests to [Google Public DNS](https://developers.google.com/speed/public-dns/docs/dns-over-https).

It listens on port 35353/udp.

It is not yet complete.  It currently supports `A`, `AAAA`, `CNAME` and `MX` records.

It is intended for use on either a single machine, or behind a better (ideally caching) DNS server like [BIND](https://www.isc.org/downloads/bind/).
