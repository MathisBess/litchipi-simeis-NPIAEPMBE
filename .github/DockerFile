FROM rust:1.85.0-bookworm

WORKDIR /usr/src/simeis

COPY . .

RUN cargo build --release

EXPOSE 8080

CMD ["./target/release/simeis"]