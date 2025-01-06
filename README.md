# memcrsd memcached server implementation in Rust

memcrsd is a key value store implementation in Rust. It is compatible with binary protocol of memcached server.

## Supported features and compatibility

To check compatibility with memcached server implementation memcrsd project
is using [memcapable](http://docs.libmemcached.org/bin/memcapable.html) tool from [libmemcached library](https://libmemcached.org/libMemcached.html)

Here is a capability status for memcrsd:

```sh
./memcapable -h 127.0.0.1  -b -p 11211
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

## Bug reports

Feel free to use the issue tracker on github.

**If you are reporting a security bug** please contact a maintainer privately.
We follow responsible disclosure: we handle reports privately, prepare a
patch, allow notifications to vendor lists. Then we push a fix release and your
bug can be posted publicly with credit in our release notes and commit
history.

## Website

* [https://memc.rs/](https://memc.rs/)

## Docker image

Docker image is available at docker hub: [https://hub.docker.com/r/memcrs/memc-rs](https://hub.docker.com/r/memcrs/memc-rs)

### Building docker image

Docker image contains only one binary, so image is pretty small(~8MB). To be able to build docker image additional memory needs to be granted to container that builds final image. Building docker image is divided in 2 stages. In stage one rust:latest image is used to compile static binary and the second stage contains just copies built binary into to final image.

To build docker image memcrsd sources have to be cloned and `docker build -m 512m .` command executed:

```sh
git clone git@github.com:memc-rs/memc-rs.git memc-rs
cd memc-rs
docker build -m 4096m .
```

### Publishing docker image

```sh
git checkout memcrsd-0.0.1b
docker pull rust
docker build -m 4096m -t memcrsd .
docker images
# tag docker image
docker tag 769dba683c8b memcrs/memc-rs:0.0.1b
docker tag memcrs/memc-rs:0.0.1b memcrs/memc-rs:latest
docker push memcrs/memc-rs:latest
docker push memcrs/memc-rs:0.0.1b
```

### Getting docker image from docker hub

To get latest version of memcrsd run following command:

```sh
docker image pull memcrs/memc-rs:latest
```

If you want specific version please take a look at available tags: [https://hub.docker.com/r/memcrs/memc-rs/tags](https://hub.docker.com/r/memcrs/memc-rs/tags)

### Runnig docker image

```sh
docker run -p 127.0.0.1:11211:11211/tcp -d memcrs/memc-rs
```

## Testing

memcrsd project is tested using different types of tests:

* unit testing,
* fuzzy testing,
* end-2-end tests

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
firefox ../target/debug/coverage/index.html
```

To be able to produce coverage reports `grcov` tool needs to be installed:

```sh
cargo install grcov
```

The plan in the future is to have coverage ~90%.

### Integration testing

For end-to-end integration testing at the moment memcrsd see tests in tests directory.

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
