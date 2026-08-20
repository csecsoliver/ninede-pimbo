# Build. SQLX_OFFLINE makes the sqlx macros read .sqlx/ instead of needing a
# live database, so this builds anywhere.
FROM docker.io/library/rust:1-trixie AS build

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /src
ENV SQLX_OFFLINE=true

# Dependencies first, so editing src/ does not rebuild the whole crate graph.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs \
    && cargo build --release --locked \
    && rm -rf src

COPY .sqlx ./.sqlx
COPY migrations ./migrations
COPY src ./src
RUN touch src/main.rs && cargo build --release --locked

# Run.
FROM docker.io/library/debian:trixie-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3 curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --create-home --uid 10001 pimbo

COPY --from=build /src/target/release/ninede-pimbo /usr/local/bin/ninede-pimbo

USER pimbo
ENV BIND=0.0.0.0:3000
EXPOSE 3000
CMD ["ninede-pimbo"]
