FROM node:22-slim AS web-build
WORKDIR /web
COPY web/package.json web/package-lock.json ./
RUN npm ci
COPY web/ ./
RUN npm run build

FROM rust:slim-bookworm AS build
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY --from=web-build /web/dist ./web/dist
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
