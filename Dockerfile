FROM rust:1.59.0-bullseye

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        build-essential \
        pkg-config \
        zlib1g-dev \
    && rm -rf /var/lib/apt/lists/*
