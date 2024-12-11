#!/usr/bin/env sh

export RUST_BACKTRACE=full
#export RUSTFLAGS="-Znew-llvm-pass-manager=no"
cargo clean
cargo fuzz run -j 8 fuzz_binary_decoder --  -rss_limit_mb=4192 -timeout=60

