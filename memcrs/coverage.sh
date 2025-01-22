#!/usr/bin/env sh

export CARGO_INCREMENTAL=0
cargo clean
cargo llvm-cov test --lib
cargo llvm-cov report --html
#grcov ../target/debug/ -s . -t html --llvm --branch --ignore-not-existing -o ../target/debug/coverage/
