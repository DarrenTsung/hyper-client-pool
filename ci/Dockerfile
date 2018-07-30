FROM osig/rust-ubuntu:1.26

RUN apt-get update && apt-get install -y git ssh lsof

WORKDIR /hyper-client-pool

RUN USER=nobody cargo init . --lib
COPY Cargo.toml .

RUN cargo build
CMD bash
