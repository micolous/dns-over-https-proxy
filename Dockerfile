FROM rust:1.23.0

WORKDIR /usr/src/dns-over-https-proxy
COPY . .

RUN cargo install

EXPOSE 35353/udp
CMD ["dns-over-https-proxy", "0.0.0.0:35353"]

