#!/usr/bin/env sh

export CARGO_INCREMENTAL=0
export RUSTFLAGS="-Cinstrument-coverage -Cprofile-generate -Ccodegen-units=1 -Copt-level=0 -Clink-dead-code -Coverflow-checks=off -Zpanic_abort_tests -Cpanic=abort"
export RUSTDOCFLAGS="-Cpanic=abort"
cargo clean
cargo +nightly test
#grcov ../target/debug/ -s . -t html --llvm --branch --ignore-not-existing -o ../target/debug/coverage/
