FROM rust:1.88.0-bookworm

WORKDIR /usr/src/simeis

COPY . .

RUN cargo update

RUN cargo build --release

EXPOSE 8080

CMD ["./target/release/simeis-server"]