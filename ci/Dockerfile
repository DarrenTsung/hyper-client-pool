FROM osig/rust-ubuntu:1.26

RUN apt-get update && apt-get install -y git ssh lsof

WORKDIR /hyper-client-pool
COPY Cargo.toml .
COPY Cargo.lock .

RUN cargo build
CMD bash
