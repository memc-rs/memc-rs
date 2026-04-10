FROM rust:latest AS builder
WORKDIR /build
COPY . /build
RUN apt-get update && apt-get install -y build-essential libevent-dev && rm -rf /var/lib/apt/lists/*
RUN cargo build
RUN cd tests/memcached && make all

FROM debian:bookworm-slim AS runtime
WORKDIR /app
RUN apt-get update && apt-get install -y libevent-2.1-7 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/tests/memcached/testapp ./testapp
COPY --from=builder /build/tests/memcached/timedrun ./tests/memcached/timedrun
COPY --from=builder /build/target/debug/memcrsd ./target/debug/memcrsd
CMD ["./testapp"]
