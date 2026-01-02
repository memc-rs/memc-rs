# Create the build container to compile the hello world program
FROM rust:latest AS builder
ENV USER=root
RUN mkdir /build
COPY . /build
WORKDIR /build
RUN rustup target add x86_64-unknown-linux-musl
RUN apt update && apt install -y musl-tools musl-dev
RUN apt install gcc
RUN cargo build --release --target=x86_64-unknown-linux-musl --features=jemallocator

# Create the execution container by copying the compiled hello world to it and running it
FROM scratch
COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/memcrsd /memcrsd
ENTRYPOINT [ "/memcrsd",  "-c", "50000", "-l", "0.0.0.0", "-v", "-m", "2048", "-i", "10m", "-s", "moka"]

