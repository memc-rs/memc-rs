# Performance comparison between memcrs and memcached (native)

This post compares memtier_benchmark results for native memcached and memcrs configured with Moka as the cache backend and Tokio current-thread runtime (i.e. multiple single thread runtimes created by each core). Both systems run natively without Docker. Metrics come from the aggregated average of 5 runs for each workload (read-heavy, write-heavy, stress). All latency values are in milliseconds.

## What is memcrs?

**memcrs** is a memory cache server rewritten in Rust, designed as a drop-in replacement for memcached. It implements the memcache protocol with pluggable cache backends (currently testing Moka, a concurrent in-memory cache library for Rust) and async runtime flexibility (Tokio). Key motivations:

- **Memory safety**: Rust eliminates entire classes of bugs (buffer overflows, use-after-free, data races) present in C-based memcached.
- **Modern abstractions**: Pluggable backends allow experimenting with different eviction policies and memory models without recompiling the protocol handler.
- **Async-first design**: Tokio async runtime enables efficient connection handling and I/O operations.
- **Production readiness**: Aims for operational parity with memcached while exploring performance trade-offs and safety improvements.

This comparison evaluates whether the safety and maintainability benefits justify any performance differences on the same hardware.

## Test setup

### Test machine

- Dell Laptop
- OS: Ubuntu 24.04.4 LTS
- Memory: 64 GB
- Processor: 11th Gen Intel(R) Core(TM) i7-11850H @ 2.50GHz (max turbo 4.80, 16 cores)

### Methodology

- Tooling: memtier_benchmark with read-heavy, write-heavy, and stress profiles, each repeated 5 times.
- Reporting: this post focuses on aggregated averages (Totals line) for throughput and latency, while also noting best and worst runs for context.
- Throughput metric: ops/sec from the Totals row (combined gets + sets).
- Latency metric: average latency from the Totals row; tail latency from p99 and p99.9.
- Environment: Both memcached and memcrs run natively (no Docker). memcrs configured with Moka cache backend and Tokio current-thread runtime.

- memcached command:

```sh
./memcached -M -B binary -t 8 -m 400
```

- memcrs command:

```sh
./memcrsd -m 50000 -t 8 -r current-thread -s moka -vv
```

- memtier_benchmark command:

```sh
docker run --mount type=bind,src=.,dst=/mnt \
  --workdir /mnt \
  --net=host -it \
  --rm redislabs/memtier_benchmark:latest \
  --port=11211 \
  --run-count=5 \
  --protocol=memcache_binary \
  --threads=6 \
  --clients=50 \
  --test-time=60 \
  --ratio=[1:10|5:5|1:10] \
  --data-size=[256|1024|512] \
  --key-maximum=[1000000|1000000|10000000] \
  --key-pattern=[R:R|R:R|G:G] \
  --hdr-file-prefix="[workload]_memcached_host" \
  --hide-histogram
```

## Summary (aggregated averages, totals)

| Workload | memcached ops/sec | memcrs ops/sec | Throughput delta | memcached avg latency | memcrs avg latency | Latency delta |
| --- | --- | --- | --- | --- | --- | --- |
| Read-heavy | 544,166.50 | 519,517.88 | -4.5% | 0.55128 | 0.57735 | +4.7% |
| Write-heavy | 503,194.40 | 448,883.02 | -10.8% | 0.59613 | 0.66813 | +12.1% |
| Stress | 519,091.07 | 468,647.06 | -9.7% | 0.57791 | 0.63998 | +10.7% |

Notes on deltas: negative throughput delta favors memcached; positive latency delta favors memcached (lower is better).

## Detailed comparison

### Read-heavy workload

- Throughput: memcached is ~4.5% higher (544.2k vs 519.5k ops/sec).
- Latency: memcached is ~4.7% lower average latency (0.551 ms vs 0.577 ms).
- Tail latency: memcached is lower at p99 (0.839 vs 0.927 ms) and p99.9 (1.078 vs 1.183 ms).
- Best/worst context: memcrs ranges 474.0k to 571.1k ops/sec (avg latency 0.525 to 0.633 ms). memcached ranges 522.5k to 602.7k ops/sec (avg latency 0.498 to 0.574 ms).

### Write-heavy workload

- Throughput: memcached is ~10.8% higher (503.2k vs 448.9k ops/sec).
- Latency: memcached is ~12.1% lower average latency (0.596 ms vs 0.668 ms).
- Tail latency: memcached is better at p99 (0.983 vs 1.015 ms) and p99.9 (1.255 vs 1.591 ms).
- Best/worst context: memcrs ranges 442.8k to 455.0k ops/sec (avg latency 0.657 to 0.659 ms). memcached ranges 499.9k to 504.7k ops/sec (avg latency 0.594 to 0.600 ms).

### Stress workload

- Throughput: memcached is ~9.7% higher (519.1k vs 468.6k ops/sec).
- Latency: memcached is ~10.7% lower average latency (0.578 ms vs 0.640 ms).
- Tail latency: memcached is better at p99 (0.951 vs 0.991 ms) and p99.9 (1.335 vs 1.463 ms).
- Best/worst context: memcrs ranges 457.6k to 478.7k ops/sec (avg latency 0.626 to 0.655 ms). memcached ranges 515.0k to 521.9k ops/sec (avg latency 0.575 to 0.583 ms).

## Takeaways

- memcached leads consistently across all workloads (read-heavy, write-heavy, stress) in both throughput and latency.
- Read-heavy: 4.5% throughput advantage, 4.7% latency advantage for memcached.
- Write-heavy: 10.8% throughput advantage, 12.1% latency advantage for memcached (largest gap).
- Stress test: 9.7% throughput advantage, 10.7% latency advantage for memcached.
- memcrs shows more variance in write-heavy workload (ranges 442.8k-455.0k); memcached is more consistent (499.9k-504.7k).

## Limitations and Notes

- Both systems run natively on the same test machine, eliminating Docker-related overhead differences.
- Memory configurations differ: memcached uses 400MB (-m 400), memcrs uses 50000 cache size, reflecting different memory models (memcached uses MB, memcrs uses item count).
- memcrs optimization opportunities: Tokio runtime tuning, Moka eviction policy tuning, thread count adjustment.

## HDR Histogram Latency Analysis (stress workload)

Histogram files available:

- memcached stress runs: `stress_test_memcached_host_run__FULL_RUN_[1-5].{txt,hgrm}`
- memcrs stress runs: `stress_test_memcrs_moka_current_run__FULL_RUN_[1-5].{txt,hgrm}`

**Per-run HDR summary (mean/stddev/max, ms):**

| Run | memcached host | memcrs moka |
| --- | --- | --- |
| 1 | 0.575 / 0.118 / 6.943 | 0.655 / 0.115 / 7.039 |
| 2 | 0.575 / 0.118 / 6.559 | 0.627 / 0.113 / 6.559 |
| 3 | 0.576 / 0.119 / 7.519 | 0.646 / 0.116 / 6.463 |
| 4 | 0.581 / 0.123 / 9.407 | 0.627 / 0.111 / 6.783 |
| 5 | 0.583 / 0.125 / 9.215 | 0.646 / 0.118 / 6.495 |

**Across-run bands (min / avg / max, ms):**

| Metric | memcached host | memcrs moka |
| --- | --- | --- |
| mean | 0.575 / 0.578 / 0.583 | 0.627 / 0.640 / 0.655 |
| stddev | 0.118 / 0.120 / 0.125 | 0.111 / 0.115 / 0.118 |
| max | 6.559 / 7.877 / 9.407 | 6.463 / 6.668 / 7.039 |

**Observations:**

- memcached host shows tighter mean band (0.575-0.583 ms) with consistent stddev (0.118-0.125 ms).
- memcrs moka shows wider mean variance (0.627-0.655 ms) with lower stddev consistency.
- memcached host has spikier tail latencies (max range 6.559-9.407 ms) in runs 4-5.
- memcrs moka shows more consistent maximum latencies (6.463-7.039 ms) across all runs.

## Appendix: raw best/worst tables

### Read-heavy workload (memcrs)

BEST RUN RESULTS

```text
Type         Ops/sec     Hits/sec   Misses/sec    Avg. Latency     p50 Latency     p99 Latency   p99.9 Latency       KB/sec
----------------------------------------------------------------------------------------------------------------------------
Sets        51924.48          ---          ---         0.52872         0.51100         0.88700         1.21500     15307.73
Gets       519218.95      2621.16    516597.78         0.52485         0.51100         0.87900         1.13500     19211.54
Waits           0.00          ---          ---             ---             ---             ---             ---          ---
Totals     571143.43      2621.16    516597.78         0.52520         0.51100         0.87900         1.14300     34519.27
```

WORST RUN RESULTS

```text
Type         Ops/sec     Hits/sec   Misses/sec    Avg. Latency     p50 Latency     p99 Latency   p99.9 Latency       KB/sec
----------------------------------------------------------------------------------------------------------------------------
Sets        43097.24          ---          ---         0.63601         0.61500         0.97500         1.35900     12705.40
Gets       430947.14      4539.39    426407.75         0.63236         0.61500         0.96700         1.36700     15945.76
Waits           0.00          ---          ---             ---             ---             ---             ---          ---
Totals     474044.38      4539.39    426407.75         0.63269         0.61500         0.96700         1.36700     28651.16
```

### Read-heavy workload (memcached)

BEST RUN RESULTS

```text
Type         Ops/sec     Hits/sec   Misses/sec    Avg. Latency     p50 Latency     p99 Latency   p99.9 Latency       KB/sec
----------------------------------------------------------------------------------------------------------------------------
Sets        54788.84          ---          ---         0.49853         0.51900         0.83900         1.08700     16152.19
Gets       547863.25    258787.15    289076.10         0.49777         0.51900         0.83900         1.07100     20271.26
Waits           0.00          ---          ---             ---             ---             ---             ---          ---
Totals     602652.09    258787.15    289076.10         0.49784         0.51900         0.83900         1.07900     36423.46
```

WORST RUN RESULTS

```text
Type         Ops/sec     Hits/sec   Misses/sec    Avg. Latency     p50 Latency     p99 Latency   p99.9 Latency       KB/sec
----------------------------------------------------------------------------------------------------------------------------
Sets        47504.22          ---          ---         0.57487         0.59900         0.94300         1.20700     14004.57
Gets       475017.13    250921.64    224095.50         0.57402         0.59900         0.94300         1.22300     17576.21
Waits           0.00          ---          ---             ---             ---             ---             ---          ---
Totals     522521.35    250921.64    224095.50         0.57409         0.59900         0.94300         1.22300     31580.77
```

### Write-heavy workload (memcrs)

BEST RUN RESULTS

```text
Type         Ops/sec     Hits/sec   Misses/sec    Avg. Latency     p50 Latency     p99 Latency   p99.9 Latency       KB/sec
----------------------------------------------------------------------------------------------------------------------------
Sets       227482.54          ---          ---         0.66093         0.62300         0.99900         1.67100    237676.73
Gets       227469.13     27009.11    200460.02         0.65753         0.62300         0.99900         1.64700      8416.78
Waits           0.00          ---          ---             ---             ---             ---             ---          ---
Totals     454951.67     27009.11    200460.02         0.65923         0.62300         0.99900         1.65500    246093.50
```

WORST RUN RESULTS

```text
Type         Ops/sec     Hits/sec   Misses/sec    Avg. Latency     p50 Latency     p99 Latency   p99.9 Latency       KB/sec
----------------------------------------------------------------------------------------------------------------------------
Sets       221416.42          ---          ---         0.67914         0.63900         1.03100         1.61500    231338.77
Gets       221404.54     14804.44    206600.10         0.67540         0.63900         1.02300         1.64700      8192.40
Waits           0.00          ---          ---             ---             ---             ---             ---          ---
Totals     442820.96     14804.44    206600.10         0.67727         0.63900         1.03100         1.63100    239531.17
```

### Write-heavy workload (memcached)

BEST RUN RESULTS

```text
Type         Ops/sec     Hits/sec   Misses/sec    Avg. Latency     p50 Latency     p99 Latency   p99.9 Latency       KB/sec
----------------------------------------------------------------------------------------------------------------------------
Sets       252350.23          ---          ---         0.59537         0.62300         0.97500         1.25500    263658.74
Gets       252337.78     35911.57    216426.21         0.59335         0.62300         0.97500         1.23900      9336.86
Waits           0.00          ---          ---             ---             ---             ---             ---          ---
Totals     504688.02     35911.57    216426.21         0.59436         0.62300         0.97500         1.24700    272995.59
```

WORST RUN RESULTS

```text
Type         Ops/sec     Hits/sec   Misses/sec    Avg. Latency     p50 Latency     p99 Latency   p99.9 Latency       KB/sec
----------------------------------------------------------------------------------------------------------------------------
Sets       249959.53          ---          ---         0.60100         0.62300         0.98300         1.27100    261160.91
Gets       249947.42     35551.79    214395.63         0.59908         0.62300         0.98300         1.27100      9248.42
Waits           0.00          ---          ---             ---             ---             ---             ---          ---
Totals     499906.95     35551.79    214395.63         0.60004         0.62300         0.98300         1.27100    270409.33
```

### Stress workload (memcrs)

BEST RUN RESULTS

```text
Type         Ops/sec     Hits/sec   Misses/sec    Avg. Latency     p50 Latency     p99 Latency   p99.9 Latency       KB/sec
----------------------------------------------------------------------------------------------------------------------------
Sets        43518.48          ---          ---         0.63006         0.60700         0.97500         1.43100     23756.36
Gets       435159.07       864.89    434294.18         0.62624         0.60700         0.96700         1.40700     16570.26
Waits           0.00          ---          ---             ---             ---             ---             ---          ---
Totals     478677.55       864.89    434294.18         0.62658         0.60700         0.96700         1.40700     40326.63
```

WORST RUN RESULTS

```text
Type         Ops/sec     Hits/sec   Misses/sec    Avg. Latency     p50 Latency     p99 Latency   p99.9 Latency       KB/sec
----------------------------------------------------------------------------------------------------------------------------
Sets        41604.14          ---          ---         0.65922         0.63100         1.00700         1.52700     22711.34
Gets       416018.32       505.46    415512.87         0.65499         0.63100         1.00700         1.52700     15841.40
Waits           0.00          ---          ---             ---             ---             ---             ---          ---
Totals     457622.46       505.46    415512.87         0.65538         0.63100         1.00700         1.52700     38552.74
```

### Stress workload (memcached)

BEST RUN RESULTS

```text
Type         Ops/sec     Hits/sec   Misses/sec    Avg. Latency     p50 Latency     p99 Latency   p99.9 Latency       KB/sec
----------------------------------------------------------------------------------------------------------------------------
Sets        47443.28          ---          ---         0.57613         0.59900         0.95100         1.23900     25898.87
Gets       474407.12      1103.11    473304.01         0.57473         0.59900         0.94300         1.22300     18064.81
Waits           0.00          ---          ---             ---             ---             ---             ---          ---
Totals     521850.39      1103.11    473304.01         0.57486         0.59900         0.94300         1.22300     43963.68
```

WORST RUN RESULTS

```text
Type         Ops/sec     Hits/sec   Misses/sec    Avg. Latency     p50 Latency     p99 Latency   p99.9 Latency       KB/sec
----------------------------------------------------------------------------------------------------------------------------
Sets        46816.46          ---          ---         0.58382         0.60700         0.95900         1.57500     25556.70
Gets       468139.28      1089.40    467049.89         0.58242         0.60700         0.95900         1.57500     17826.13
Waits           0.00          ---          ---             ---             ---             ---             ---          ---
Totals     514955.74      1089.40    467049.89         0.58255         0.60700         0.95900         1.57500     43382.83
```
