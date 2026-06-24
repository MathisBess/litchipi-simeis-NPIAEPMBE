FROM rust:bookworm

WORKDIR /usr/src/simeis

COPY . .

RUN cargo build --release

EXPOSE 8081

CMD ["./target/release/simeis-server"]