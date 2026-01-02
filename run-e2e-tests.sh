#!/bin/bash

set -e

echo "Running e2e tests..."

if [ "$1" = "clean" ]; then
    echo "Cleaning build artifacts..."
    cargo clean
fi

echo "Running tests..."
if ! cargo test --test '*' -- --nocapture; then
    echo "E2E tests failed!"
    exit 1
fi



echo "Building Docker image..."
if ! docker build -m 4096m . -t memcrs/memc-rs:dev; then
    echo "Docker build failed!"
    exit 1
fi

echo "Starting Docker container..."
if ! docker run -d --cidfile name.txt -p 127.0.0.1:11211:11211/tcp memcrs/memc-rs:dev; then
    echo "Docker run failed!"
    exit 1
fi

echo "Running memcapable tests..."
docker run --rm --network host memcrs/memcached-awesome:latest

echo "Stopping Docker container..."
docker stop `cat name.txt`
rm name.txt

echo "E2E tests completed successfully!"

