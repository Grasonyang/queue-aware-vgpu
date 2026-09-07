FROM rust:1.94-bookworm AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --bin queue-aware-vgpu-controller

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install --yes --no-install-recommends ca-certificates \
    && apt-get clean \
    && rm --recursive --force /var/lib/apt/lists/*
COPY --from=build /src/target/release/queue-aware-vgpu-controller /usr/local/bin/queue-aware-vgpu-controller
USER 65532:65532
ENTRYPOINT ["/usr/local/bin/queue-aware-vgpu-controller"]
