# ---------- Build stage ----------
FROM rust:1.94-bookworm AS builder

WORKDIR /app

COPY . .

RUN cargo build --release


# ---------- Runtime stage ----------
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        chromium \
        ca-certificates \
        fonts-liberation \
        fonts-noto-color-emoji \
        && \
    rm -rf /var/lib/apt/lists/*

RUN useradd -m -u 1000 appuser

WORKDIR /app

COPY --from=builder /app/target/release/rust-web-reader /app/rust-web-reader

RUN chown appuser:appuser /app/rust-web-reader

USER appuser

ENV CHROME_PATH=/usr/bin/chromium

EXPOSE 10000

CMD ["/app/rust-web-reader"]