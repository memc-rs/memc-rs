#!/usr/bin/env sh

export RUST_BACKTRACE=full
cargo +nightly fuzz run -j 8 fuzz_binary_encoder --  -rss_limit_mb=4192 -timeout=60

