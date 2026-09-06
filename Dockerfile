FROM rust:slim-bookworm AS build
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --locked

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ffmpeg ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=build /build/target/release/yarrtube /usr/local/bin/yarrtube

WORKDIR /app

EXPOSE 8080
VOLUME ["/videos"]

ENTRYPOINT ["yarrtube"]
CMD ["serve"]
