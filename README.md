# memcrsd memcached server implementation in Rust

memcrsd is a key value store implementation in Rust. It is compatible with binary protocol of memcached server.

## Changelog (since memcrsd-0.0.1m)

**Important:** Configuration options for Moka and DashMap store engines are now separated. Options like `--max-capacity` and `--eviction-policy` apply only to Moka, while `--memory-limit` applies only to DashMap. This makes configuration clearer and prevents confusion when switching between store engines.

## Supported features and compatibility

To check compatibility with memcached server implementation memcrsd project
is using [memcapable](https://awesomized.github.io/libmemcached/bin/memcapable.html) tool from [libmemcached-awesome library](https://github.com/awesomized/libmemcached)

Here is a capability status for memcrsd:

```sh
docker run --rm --network host memcrs/memcached-awesome:latest
Hostname was provided.127.0.0.1
binary noop                             [pass]
binary quit                             [pass]
binary quitq                            [pass]
binary set                              [pass]
binary setq                             [pass]
binary flush                            [pass]
binary flushq                           [pass]
binary add                              [pass]
binary addq                             [pass]
binary replace                          [pass]
binary replaceq                         [pass]
binary delete                           [pass]
binary deleteq                          [pass]
binary get                              [pass]
binary getq                             [pass]
binary getk                             [pass]
binary getkq                            [pass]
binary incr                             [pass]
binary incrq                            [pass]
binary decr                             [pass]
binary decrq                            [pass]
binary version                          [pass]
binary append                           [pass]
binary appendq                          [pass]
binary prepend                          [pass]
binary prependq                         [pass]
binary stat                             [pass]
All tests passed
```

Here is a memcached integration tests output:

```sh
ok 1 - start_server
ok 2 - binary_noop
ok 3 - binary_set
ok 4 - binary_setq
ok 5 - binary_add
ok 6 - binary_addq
ok 7 - binary_replace
ok 8 - binary_replaceq
ok 9 - binary_delete
ok 10 - binary_deleteq
ok 11 - binary_get
ok 12 - binary_getq
ok 13 - binary_getk
ok 14 - binary_getkq
ok 15 - binary_incr
ok 16 - binary_incrq
ok 17 - binary_decr
ok 18 - binary_decrq
ok 19 - binary_version
ok 20 - binary_flush
ok 21 - binary_flushq
ok 22 - binary_append
ok 23 - binary_appendq
ok 24 - binary_prepend
ok 25 - binary_prependq
ok 26 - binary_illegal
ok 27 - binary_pipeline_hickup
ok 28 - stop_server
```

## Bug reports

Feel free to use the issue tracker on github.

**If you are reporting a security bug** please contact a maintainer privately.
We follow responsible disclosure: we handle reports privately, prepare a
patch, allow notifications to vendor lists. Then we push a fix release and your
bug can be posted publicly with credit in our release notes and commit
history.

## Website

* [https://memc.rs/](https://memc.rs/)

## Running

`memcrsd` is configured via command-line options. Most options have sensible defaults; below are the primary flags and their meanings (see the `--help` output for the full list).

* `-p, --port <PORT>`: TCP port the server will bind to for incoming connections. Default: `11211`.

* `-c, --connection-limit <CONNECTION-LIMIT>`: maximum number of simultaneous client connections allowed. Default: `1024`.

* `-b, --backlog-limit <LISTEN-BACKLOG>`: backlog queue size for pending TCP connections. Default: `1024`.

* `-i, --item-size-limit <MAX-ITEM-SIZE>`: maximum allowed size for a single item (between 1KiB and 1024MiB). Default: `1MiB`.

* `-t, --threads <THREADS>`: number of worker threads (defaults to number of CPU cores). Default: `8`.

* `-v, --verbose...`: increase log verbosity (can be repeated).

* `-l, --listen-address <LISTEN-ADDRESS>`: IP address or interface the server will bind to. Default: `127.0.0.1`.

* `-r, --runtime-type <RUNTIME-TYPE>`: execution runtime: `current-thread` or `multi-threaded`.

  Possible values:
  - `current-thread`: every thread will create its own runtime which will handle work without thread switching
  - `multi-threaded`:   work stealing threadpool runtime

  Default: `current-thread`.

* `--cpu-no-pin`: do not pin worker threads to cores (only for `current-thread` runtime).

* `-s, --store-engine <STORE-ENGINE>`: which underlying storage engine to use. Available options:

  - `dash-map` – use the DashMap-based memory store
  - `moka`     – use the Moka-based memory store (default)

  Default: `moka`.

* `--max-capacity <CAPACITY>`: maximum Moka cache capacity (key->value pairs). Default: `1048576`.

* `--eviction-policy <EVICTION-POLICY>`: eviction policy to use.

  Possible values:
  - `tiny-lfu`: tiny LFU,
  - `lru`: least recently used (default).

  Default: `least-recently-used`.

* `--memory-limit <MEMORY-LIMIT>`: memory limit in megabytes. Default: `64MiB`.

* `-h, --help`: Print help (see a summary with '-h').

* `-V, --version`: Print version.

Notes:

* Size values accept suffixes (examples: `1MiB`, `10k`).
* Some defaults (e.g. thread count or OS limits on connections) may be influenced by the host system.
* `--max-capacity` and `--eviction-policy` are only applicable when `--store-engine` is set to `moka`. When using `dash-map`, these options will cause an error.
* `--memory-limit` is only applicable when `--store-engine` is set to `dash-map` (it controls memory usage in megabytes). When using `moka`, control cache size with `--max-capacity`; `--memory-limit` will cause an error for `moka`.

## Docker image

For information about building, publishing, and running the Docker image, see [DOCKER.md](DOCKER.md).

## Testing

memcrsd project is tested using different types of tests:

* unit testing,
* fuzzy testing,
* end-2-end tests

### Unit testing

```sh
cargo test --lib -- --nocapture
```

### Fuzzy testing

At the moment decoding network packets is fuzzy tested.

```sh
cargo install -f cargo-fuzz
cargo +nightly fuzz build
cargo +nightly fuzz run  -j 8 fuzz_binary_decoder --  -rss_limit_mb=4192 -timeout=60
cargo +nightly fuzz coverage fuzz_binary_decoder
```

Workaround to see fuzzing test results:

```sh
~/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin/llvm-cov show fuzz/target/x86_64-unknown-linux-gnu/release/fuzz_binary_decoder  --format=html -Xdemangler=rustfilt --ignore-filename-regex="\.cargo" -instr-profile=fuzz/coverage/fuzz_binary_decoder/coverage.profdata > index.html
```

### Generating test coverage reporting

To generate test coverage there is a convenient shell script that compiles and executed unit tests:

```sh
cd memcrs
./coverage.sh
firefox ./target/llvm-cov/html/index.html
```

Coverage reporting is generated using cargo llvm-cov tool. Alternatively one can execute it from shell:

```sh
cargo llvm-cov test --lib && cargo llvm-cov report --html
```

The plan in the future is to have coverage ~90%.

### Integration testing

For end-to-end integration testing we are using the Rust client library memcache[https://crates.io/crates/memcache]. See tests directory for further details.

```sh
cargo test --test '*' -- --nocapture
```

A subset of memcached integration tests is executed, restricted to the binary protocol commands currently supported by memcrsd. These compatibility checks are driven through `testapp.Dockerfile`, which builds a helper container for the supported command set.

```sh
docker build -f testapp.Dockerfile -t memc-testapp .
docker run --rm -it memc-testapp
```

To run regression tests with a precompiled memcapable binary (on x86-64 architecture) from [https://github.com/awesomized/libmemcached](libmemcached-awesome), you can use the following Docker command:

```sh
docker run --rm --network host memcrs/memcached-awesome:latest
```

## Measuring performance

Measuring performance can be tricky, thats why to measure performance memcrsd
project is using industry standard benchmarking tool for measuring performance
of memcached server which is memtier_benchmark.
This tool can be used to generate various traffic patterns. It provides a robust
set of customization and reporting capabilities all wrapped into a convenient and
easy-to-use command-line interface.
More information about memtier benchmark tool can be found on [RedisLabs blog.](https://redislabs.com/blog/memtier_benchmark-a-high-throughput-benchmarking-tool-for-redis-memcached/)

### Memtier benchmark installation

Memtier benchmark is available on github, it needs to be cloned and compiled:

```sh
git clone https://github.com/RedisLabs/memtier_benchmark.git
autoreconf -ivf
./configure
make
make install
```

### Generating flamegraph

To be able to sample and resolve Kernel functions:

* we need to expose kernel addresses([kptr_restrict](https://sysctl-explorer.net/kernel/kptr_restrict/))
* grant unprivileged users access to performance events in kernel([perf_event_paranoid](https://sysctl-explorer.net/kernel/perf_event_paranoid/))

```sh
sudo sh -c " echo 0 > /proc/sys/kernel/kptr_restrict"
sudo sh -c " echo -1 >> /proc/sys/kernel/perf_event_paranoid"
```

Generating flamegraphs:

```sh
sudo apt install -y linux-tools-common linux-tools-generic
cargo install flamegraph
cd ~/projects/memix
cargo flamegraph
cargo flamegraph --bin memcrsd
```

### Attaching perf

By default release profile is built with debug symbols, see Cargo.toml:

```toml
[profile.release]
opt-level = 3
debug=true
```

On Ubuntu install required packages:

```sh
sudo apt install -y linux-tools-common linux-tools-generic
```

We can start a server:

```sh
./target/release/memcrsd -v & perf record -F 99 -p `pgrep memcrsd`
```

* First we run the memcrsd server and we send it to the background using the ampersand (&) symbol.
* Next to it, so it executes immediately, we run perf that receives the process identifier (PID) courtesy of pgrep memcrsd.
* The pgrep command returns the PID of a process by name.

After benchmarks are executed reports can be displayed using perf report:

```sh
perf report
```
