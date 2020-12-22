

Generating flamegraph

Kernel config

```sh
sudo sh -c " echo 0 > /proc/sys/kernel/kptr_restrict"
sudo sh -c " echo -1 >> /proc/sys/kernel/perf_event_paranoid"
```

```sh
sudo apt install -y linux-tools-common linux-tools-generic
cargo install flamegraph
cd ~/projects/memix
cargo flamegraph
cargo flamegraph --bin memcrsd

```
