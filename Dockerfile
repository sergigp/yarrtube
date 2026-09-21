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
COPY migrations ./migrations
COPY --from=web-build /web/dist ./web/dist
RUN cargo build --release --locked

# Bundles a known-good yt-dlp binary into the image at build time, so a
# freshly started container is never without a working one — the runtime
# self-update (cli/ytdlp_update.rs) is a best-effort upgrade over this floor,
# not the only source. Kept in its own stage so `curl` never lands in the
# final runtime image.
FROM debian:bookworm-slim AS ytdlp-fetch
ARG TARGETARCH
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*
RUN case "${TARGETARCH}" in \
      amd64) YTDLP_SUFFIX="" ;; \
      arm64) YTDLP_SUFFIX="_aarch64" ;; \
      *) echo "Unsupported architecture: ${TARGETARCH}" >&2; exit 1 ;; \
    esac \
    && curl -fL -o /tmp/yt-dlp "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_linux${YTDLP_SUFFIX}" \
    && chmod +x /tmp/yt-dlp

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ffmpeg ca-certificates gosu \
    && rm -rf /var/lib/apt/lists/*

COPY --from=build /build/target/release/yarrtube /usr/local/bin/yarrtube
COPY --from=ytdlp-fetch /tmp/yt-dlp /app/bin/yt-dlp
COPY docker-entrypoint.sh /usr/local/bin/docker-entrypoint.sh
RUN chmod +x /usr/local/bin/docker-entrypoint.sh

WORKDIR /app

EXPOSE 8080
VOLUME ["/videos"]

ENTRYPOINT ["docker-entrypoint.sh"]
CMD ["yarrtube", "serve"]
