FROM osig/rust-ubuntu:1.26 as build_deps
RUN apt-get update && apt-get install -y git ssh lsof
WORKDIR /hyper-client-pool
RUN USER=nobody cargo init . --lib
COPY Cargo.toml .
RUN cargo build --tests

FROM build_deps
WORKDIR /hyper-client-pool
COPY . .
CMD bash
