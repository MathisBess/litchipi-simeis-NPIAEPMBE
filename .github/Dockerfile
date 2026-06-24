FROM rust:bookworm

WORKDIR /usr/src/simeis

COPY . .

RUN cargo build --release

EXPOSE 8080

CMD ["./target/release/simeis-server"]