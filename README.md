# memc.rs pure memcache implementation in Rust

The main purpose of this project is to provide pure memcache server implementation in Rust.

## Measuring performance

To be able to measure performance memtier benchmarking tool is used from RedisLabs.

```sh
git clone https://github.com/RedisLabs/memtier_benchmark.git
autoreconf -ivf
./configure
make
make install
```

### Memtier benchmark


### Generating flamegraph

To be able to probe Kernel functions it has to be enabled:

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

Perf
```sh
./target/release/memcrsd -v & perf record -F 99 -p `pgrep memcrsd`

# after 

perf report
```
