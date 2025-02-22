# syntax=docker/dockerfile:1

# Comments are provided throughout this file to help you get started.
# If you need more help, visit the Dockerfile reference guide at
# https://docs.docker.com/go/dockerfile-reference/

# Want to help us make this template better? Share your feedback here: https://forms.gle/ybq9Krt8jtBL3iCk7

ARG RUST_VERSION=1.83.0
ARG APP_NAME=discord_selfbot

################################################################################
# Create a stage for building the application.

FROM rust:${RUST_VERSION}-alpine AS build
ARG APP_NAME
WORKDIR /app

# Install host build dependencies.
RUN apk add --no-cache clang lld musl-dev git openssl openssl-dev openssl-libs-static

# Build the application.
# Leverage a cache mount to /usr/local/cargo/registry/
# for downloaded dependencies, a cache mount to /usr/local/cargo/git/db
# for git repository dependencies, and a cache mount to /app/target/ for
# compiled dependencies which will speed up subsequent builds.
# Leverage a bind mount to the src directory to avoid having to copy the
# source code into the container. Once built, copy the executable to an
# output directory before the cache mounted /app/target is unmounted.
RUN --mount=type=bind,source=src,target=src \
    --mount=type=bind,source=assets,target=assets \
    --mount=type=bind,source=Cargo.toml,target=Cargo.toml \
    --mount=type=bind,source=Cargo.lock,target=Cargo.lock \
    --mount=type=cache,target=/app/target/ \
    --mount=type=cache,target=/usr/local/cargo/git/db \
    --mount=type=cache,target=/usr/local/cargo/registry/ \
cargo build --locked --release && \
cp ./target/release/$APP_NAME /bin/server

################################################################################
# Create a new stage for running the application that contains the minimal
# runtime dependencies for the application. This often uses a different base
# image from the build stage where the necessary files are copied from the build
# stage.
#
# The example below uses the alpine image as the foundation for running the app.
# By specifying the "3.18" tag, it will use version 3.18 of alpine. If
# reproducability is important, consider using a digest
# (e.g., alpine@sha256:664888ac9cfd28068e062c991ebcff4b4c7307dc8dd4df9e728bedde5c449d91).
FROM alpine:3.21 AS final

USER root

# Install runtime dependencies.
RUN apk add --no-cache yt-dlp yt-dlp-core ffmpeg ffmpeg-libs libwebp libwebp-tools libjxl libjxl-tools libavif libavif-apps tesseract-ocr tesseract-ocr-data-eng geckodriver firefox sudo

# Install fonts.
RUN apk add --no-cache font-terminus font-inconsolata font-dejavu font-noto font-noto-cjk font-awesome font-noto-extra font-vollkorn font-misc-cyrillic font-mutt-misc font-screen-cyrillic font-winitzki-cyrillic font-cronyx-cyrillic font-noto-thai font-noto-tibetan font-ipa font-sony-misc font-jis-misc font-isas-misc font-arabic-misc font-noto-arabic font-noto-armenian font-noto-cherokee font-noto-devanagari font-noto-ethiopic font-noto-georgian font-noto-hebrew font-noto-lao font-noto-malayalam font-noto-tamil font-noto-thaana font-twemoji

# Create a non-privileged user that the app will run under.
# See https://docs.docker.com/go/dockerfile-user-best-practices/
# ARG UID=10001
# RUN adduser \
#     --disabled-password \
#     --gecos "" \
#     --home "/nonexistent" \
#     --shell "/sbin/nologin" \
#     --no-create-home \
#     --uid "${UID}" \
#     # && echo "user ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers
#     appuser
# USER appuser

# Copy the executable from the "build" stage.
COPY --from=build /bin/server /bin/

# Expose the port that the application listens on.
# EXPOSE 2205

# ENV DISPLAY=:0

# What the container should run when it is started.
CMD sudo geckodriver & /bin/server
