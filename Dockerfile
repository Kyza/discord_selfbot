ARG RUST_VERSION=1.86.0-stable
ARG APP_NAME=discord_selfbot

FROM clux/muslrust:${RUST_VERSION} AS chef
USER root
RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
RUN cargo binstall --no-confirm cargo-chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
ARG APP_NAME
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json
COPY . .
RUN cargo build --release --target x86_64-unknown-linux-musl --bin ${APP_NAME}

FROM alpine:3.21 AS final
ARG APP_NAME
ENV APP_NAME=${APP_NAME}

# Install runtime dependencies.
RUN apk add --no-cache yt-dlp=2025.03.31-r0 yt-dlp-core=2025.03.31-r0 ffmpeg ffmpeg-libs libwebp libwebp-tools libjxl libjxl-tools libavif libavif-apps tesseract-ocr tesseract-ocr-data-eng geckodriver firefox sudo

# Install fonts.
RUN apk add --no-cache font-terminus font-inconsolata font-dejavu font-noto font-noto-cjk font-awesome font-noto-extra font-vollkorn font-misc-cyrillic font-mutt-misc font-screen-cyrillic font-winitzki-cyrillic font-cronyx-cyrillic font-noto-thai font-noto-tibetan font-ipa font-sony-misc font-jis-misc font-isas-misc font-arabic-misc font-noto-arabic font-noto-armenian font-noto-cherokee font-noto-devanagari font-noto-ethiopic font-noto-georgian font-noto-hebrew font-noto-lao font-noto-malayalam font-noto-tamil font-noto-thaana font-twemoji

COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/${APP_NAME} /usr/local/bin/

RUN addgroup -S appuser && adduser -S appuser -G appuser
USER appuser

CMD geckodriver & /usr/local/bin/${APP_NAME}
