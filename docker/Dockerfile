FROM debian:bookworm AS base
SHELL ["/bin/bash", "-c"]
RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
  libpq-dev libsqlite3-dev ssl-cert ca-certificates curl apt-transport-https \
  lsb-release file git-core build-essential libssl-dev libssl3 libgexiv2-dev \
  cmake clang ffmpeg nasm pkg-config \
  && apt-get clean && rm -rf /var/lib/apt/lists/*

FROM base AS rust-setup
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"
RUN rustup +nightly default

FROM rust-setup AS dependencies
WORKDIR /opt/build
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
# Create dummy main.rs to build dependencies only
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --no-default-features -F pg --bin enigmatick --release
# Remove dummy files but keep compiled dependencies
RUN rm -rf src target/release/enigmatick* target/release/build/enigmatick*

FROM dependencies AS builder
# Copy source and assets - this layer rebuilds on code changes
COPY src ./src
COPY client ./client
COPY bundled ./bundled
COPY migrations.pg ./migrations.pg
# Build only your code - dependencies already compiled
RUN cargo build --no-default-features -F pg --bin enigmatick --release

FROM debian:bookworm-slim AS runtime
RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
  ssl-cert \
  ca-certificates \
  libgexiv2-dev \
  libpq-dev \
  && apt-get clean && rm -rf /var/lib/apt/lists/*

RUN mkdir -p /srv/enigmatick
WORKDIR /srv/enigmatick
COPY --from=builder /opt/build/target/release/enigmatick ./

COPY docker/files/start /usr/local/bin/start
RUN chmod a+x /usr/local/bin/start

EXPOSE 8010

CMD ["/usr/local/bin/start"]
