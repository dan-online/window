# Chef dependencies
FROM rust as planner
WORKDIR app

RUN cargo install cargo-chef 
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Install dependencies
FROM rust as cacher
WORKDIR app
RUN cargo install cargo-chef
COPY --from=planner /app/recipe.json recipe.json
RUN apt update -y
RUN apt install -y clang libavcodec-dev libavdevice-dev libavfilter-dev libavformat-dev libavutil-dev libpostproc-dev libswresample-dev libswscale-dev
RUN cargo chef cook --release --recipe-path recipe.json

# Build ytt
FROM rust:1.79.0 as builder

WORKDIR /usr/src/
RUN USER=root cargo new --bin ytt
WORKDIR /usr/src/ytt

# Compile dependencies
COPY Cargo.toml Cargo.lock ./

# Copy source and build
COPY src src

# Build dependencies
COPY --from=cacher /app/target target
COPY --from=cacher /usr/local/cargo /usr/local/cargo
RUN rm -rf target/release/ytt*
RUN apt install -y clang libavcodec-dev libavdevice-dev libavfilter-dev libavformat-dev libavutil-dev libpostproc-dev libswresample-dev libswscale-dev
RUN cargo build --locked --release

# Run application
FROM ubuntu:noble

WORKDIR /app

# RUN apt-get update -y && apt-get install -y clang ca-certificates libssl-dev --no-install-recommends && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/ytt/target/release/ytt /usr/local/bin/

CMD ["/usr/local/bin/ytt"]