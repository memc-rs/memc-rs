# ARM Performance Comparison: Memcached vs Memcrs

## Executive Summary

In this comprehensive benchmark analysis, we compare the performance characteristics of the traditional **Memcached** against **Memcrs** (a Rust implementation of memcached configured with Moka cache backend and multiple Tokio runtime configurations). Both systems were tested on an ARM-based SBC to provide real-world performance insights for edge computing and embedded systems scenarios.

### Key Findings

**With Single-Threaded Tokio Runtime:**

- Memcached achieves 60-85% higher throughput
- Memcrs single-threaded shows significant tail latency issues

**With Multi-Threaded Tokio Runtime:**

- Memcrs throughput improves by 56%, nearly matching Memcached (94.4% parity)
- **Memcrs multi-threaded achieves BETTER tail latencies (p99.9: 4.8ms vs 6.0ms)**
- Runtime configuration is critical for ARM multi-core systems
- Both systems demonstrate stable performance across runs

---

## Test Environment

### Hardware Specifications

- **CPU**: Rockchip RK3588 SoC with 8-core (Cortex-A76x4 + Cortex-A55x4) 64-bit processor (<https://docs.turingpi.com/docs/turing-rk1-specs-and-io-ports>)
- **Max Clock Speed**: 2.4 GHz
- **RAM**: 16 GB
- **Architecture**: ARM64

### Software Configuration

**Memcached:**

```bash
./memcached -m 1024 -t 4 -v
```

- Memory limit: 1024 MB
- Worker threads: 4
- Verbose output enabled

**Memcrs - Single-Threaded Version:**

```bash
./target/release/memcrsd -s moka -r current-thread -m 50000 -vv

```

- Cache backend: Moka
- Runtime: Tokio current-thread (single-threaded async)
- Item count limit: 50k
- Verbose output enabled

**Memcrs - Multi-Threaded Version:**

```bash
./target/release/memcrsd -m 50000000 -r multi-thread -s moka -vv -t 4 -c 1024
```

- Cache backend: Moka
- Runtime: Tokio multi-threaded (work-stealing scheduler)
- Memory limit: 50 milion (to eliminate evictions like in memcached case)
- Worker threads: 4
- Max connections: 1024
- Verbose output enabled

**Note**: This comparison includes results from both single-threaded and multi-threaded Memcrs configurations to demonstrate the impact of runtime architecture on ARM systems.

---

### Methodology

- Tooling: memtier_benchmark with read-heavy, write-heavy, and stress profiles, each repeated 5 times.
- Reporting: this post focuses on aggregated averages (Totals line) for throughput and latency, while also noting best and worst runs for context.
- Throughput metric: ops/sec from the Totals row (combined gets + sets).
- Latency metric: average latency from the Totals row; tail latency from p99 and p99.9.
- Environment: Both memcached and memcrs run natively (no Docker). memcrs configured with Moka cache backend and Tokio current-thread runtime.
- memtier_benchmark command (compiled directly from master):

```sh
 ./memtier_benchmark \
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

---

## Updated Analysis: Multi-Threaded Memcrs Runtime

> **Important**: The recommendations below were based on single-threaded Memcrs results. Following additional testing, we also evaluated **multi-threaded Memcrs**, which dramatically changes the performance picture. See below for revised recommendations.

Following the single-threaded analysis, additional benchmarks were executed with **Memcrs configured using Tokio's multi-threaded runtime** with 4 worker threads and 50 concurrent clients:

```bash
./target/release/memcrsd -m 50000000 -r multi-thread -s moka -vv -t 4 -c 1024
```

### Multi-Threaded vs Single-Threaded Memcrs

#### Throughput Impact

| Configuration | Avg Ops/sec | Improvement |
|---------------|-------------|-------------|
| Single-threaded | 106,027 | Baseline |
| Multi-threaded | 165,563 | **+56.2%** |

**Key Insight**: Enabling multi-threaded async runtime dramatically improves Memcrs performance, increasing throughput by over 56%.

#### Latency Characteristics (Multi-Threaded Run)

| Percentile | Latency (ms) | vs Single-threaded |
|-----------|--------------|-------------------|
| p50 | 0.718 | -0.985 ms (-57.7%) ☑️ |
| p99 | 4.14 | -0.819 ms (-16.5%) ☑️ |
| p99.9 | 4.8 | -8.281 ms (-63.3%) ☑️ |
| Mean | 1.207 | -0.681 ms (-36.0%) ☑️ |

**Significant Finding**: Multi-threaded configuration not only increases throughput but also **dramatically improves latencies across all percentiles**. The 99.9th percentile latency drops from 13.119 ms to 4.8 ms—a 2.7× improvement.

### Three-Way Performance Comparison: Stress Test Workload

| Metric | Memcached | Memcrs Single-Threaded | Memcrs Multi-Threaded | Memcached Advantage |
|--------|-----------|------------------------|----------------------|-------------------|
| **Throughput (Ops/sec)** | 175,364 | 106,027 | 165,563 | Only 5.9% edgecase |
| **p50 Latency (ms)** | 0.487 | 1.703 | 0.718 | 1.5× |
| **p99 Latency (ms)** | 4.543 | 4.959 | 4.14 | Memcrs wins by 1.1% |
| **p99.9 Latency (ms)** | 6.047 | 13.119 | 4.8 | **Memcrs wins** ✓ |
| **Mean Latency (ms)** | 1.138 | 1.888 | 1.207 | 1.06× |

#### Key Observations

1. **Throughput Gap Closes**: Multi-threaded Memcrs now achieves 94.4% of Memcached's throughput—a dramatic improvement from the previous 60.4%.

2. **Latency Inversion at Tail Percentiles**: Remarkably, **Memcrs multi-threaded shows better p99.9 latency than Memcached** (4.8 ms vs 6.047 ms). This suggests the async-multi-threaded model provides more consistent tail behavior.

3. **Sweet Spot Zone**: At p99 and below, Memcrs multi-threaded is competitive with Memcached while offering superior tail latency (p99.9+).

4. **Resource Utilization**: The multi-threaded configuration enables Memcrs to fully utilize the ARM processors' 8 cores (Cortex-A76 x4 + Cortex-A55 x4), whereas the single-threaded version was leaving performance on the table.

---

## Conclusion

This ARM-based performance comparison demonstrates that **with proper runtime configuration (multi-threaded Tokio), Memcrs becomes a viable competitor to Memcached**:

### Key Takeaways

1. **Runtime matters significantly**: A simple configuration change from single-threaded to multi-threaded async runtime yields 56% throughput improvement and 2.7× tail latency improvement.

2. **Memcached retains modest throughput edge**: 175K ops/sec vs 165K ops/sec (5.9%), but the gap is now within acceptable margins for most applications.

3. **Memcrs shows superior tail latency**: The p99.9 percentile for multi-threaded Memcrs (4.8 ms) beats Memcached (6.047 ms), a critical metric for latency-sensitive systems.

4. **Architectural choices matter**:
   - **Single-threaded async** = we need to investigate why there is a such poor scaling on multi-core ARM for multiple single thread Tokio runtimes
   - **Multi-threaded async** = scales across cores effectively, approaching C-based performance
   - **Native threads** (Memcached) = mature, low-overhead, slight throughput advantage

### Recommendations, Revised

**Choose Memcached if you need:**

- Absolute maximum throughput (6-9% higher)
- Proven stability in large-scale deployments
- Minimal operational overhead

**Choose Memcrs with multi-threaded runtime if you prioritize:**

- **Better tail latencies** (p99.9: 4.8ms vs 6.0ms)
- Type-safe Rust implementation
- Ease of extending with custom backends
- Modern async architecture for IO-bound workloads
- Memory safety guarantees
- Deployments where code maintainability equals performance

## Benchmark Results Memcached vs memcrs (multiple single threaded Tokio runtimes created)

### 1. Stress Test Workload (Balanced Set/Get Mix)

This workload represents typical cache operations in production environments with approximately 1:10 SET to GET ratio.

#### Stress Test Workload - Throughput Comparison

| Metric | Memcached | Memcrs | Difference | Memcached Advantage |
|--------|-----------|--------|------------|---------------------|
| **Total Ops/sec** | 175,364.40 | 106,027.46 | -69,336.94 | **+65.4%** |
| **SET Ops/sec** | 15,943.76 | 9,640.35 | -6,303.41 | **+65.4%** |
| **GET Ops/sec** | 159,420.64 | 96,387.11 | -63,033.53 | **+65.4%** |

#### Stress Test Workload - Latency Comparison

| Metric | Memcached | Memcrs | Difference | Winner |
|--------|-----------|--------|------------|--------|
| **Avg Latency** | 1.14 ms | 1.88 ms | -0.74 ms | Memcached |
| **p50 Latency** | 0.487 ms | 1.703 ms | -1.216 ms | Memcached |
| **p99 Latency** | 4.543 ms | 4.959 ms | -0.416 ms | Memcached |
| **p99.9 Latency** | 6.047 ms | 13.119 ms | -7.072 ms | Memcached |

**Analysis**: Memcached demonstrates superior performance across all metrics in the stress test. The median latency is approximately 3.5× lower, while the 99.9th percentile latency is more than 2× lower. This makes Memcached the clear choice for latency-sensitive applications requiring predictable tail latencies.

---

### 2. Read-Heavy Workload Memcached vs memcrs (multiple single threaded Tokio runtimes created)

Simulating applications that are primarily read-focused with minimal write operations.

#### Read-Heavy Workload - Throughput Comparison

| Metric | Memcached | Memcrs | Difference | Memcached Advantage |
|--------|-----------|--------|------------|---------------------|
| **Total Ops/sec** | 178,516.98 | 107,978.18 | -70,538.80 | **+65.3%** |
| **SET Ops/sec** | 16,230.28 | 9,817.70 | -6,412.58 | **+65.3%** |
| **GET Ops/sec** | 162,286.70 | 98,160.48 | -64,126.22 | **+65.3%** |

#### Read-Heavy Workload - Latency Comparison

| Metric | Memcached | Memcrs | Difference | Winner |
|--------|-----------|--------|------------|--------|
| **Avg Latency** | 1.12 ms | 1.85 ms | -0.73 ms | Memcached |
| **p50 Latency** | 0.479 ms | 1.719 ms | -1.240 ms | Memcached |
| **p99 Latency** | 4.479 ms | 4.255 ms | +0.224 ms | **Memcrs** |
| **p99.9 Latency** | 5.887 ms | 13.375 ms | -7.488 ms | Memcached |

**Analysis**: In read-heavy scenarios, the performance gap between the two systems remains consistent. Interestingly, Memcrs shows slightly better p99 latency (4.255 ms vs 4.479 ms), suggesting that it handles burst load slightly better in terms of near-worst-case operations. However, this advantage disappears at the p99.9 tail percentile.

---

### 3. Write-Heavy Workload Memcached vs memcrs (multiple single threaded Tokio runtimes created)

Simulating applications with predominant write operations and fewer reads.

#### Throughput Comparison - Write-Heavy Workload

| Metric | Memcached | Memcrs | Difference | Memcached Advantage |
|--------|-----------|--------|------------|---------------------|
| **Total Ops/sec** | 175,523.38 | 96,550.19 | -78,973.19 | **+81.8%** |
| **SET Ops/sec** | 87,765.86 | 48,279.20 | -39,486.66 | **+81.8%** |
| **GET Ops/sec** | 87,757.52 | 48,270.99 | -39,486.53 | **+81.8%** |

#### Latency Comparison - Write-Heavy Workload

| Metric | Memcached | Memcrs | Difference | Winner |
|--------|-----------|--------|------------|--------|
| **Avg Latency** | 1.14 ms | 2.07 ms | -0.93 ms | Memcached |
| **p50 Latency** | 0.487 ms | 1.823 ms | -1.336 ms | Memcached |
| **p99 Latency** | 4.703 ms | 6.655 ms | -1.952 ms | Memcached |
| **p99.9 Latency** | 5.919 ms | 14.463 ms | -8.544 ms | Memcached |

**Analysis**: The write-heavy workload reveals the most significant divergence between the two systems. Memcached shows an **81.8% throughput advantage**, the highest among all tested scenarios. Memcrs struggles more with write-heavy operations, particularly evident in the p99.9 latency (14.46 ms vs 5.92 ms), representing a 2.44× degradation. This suggests that the single-threaded async runtime may become a bottleneck under high write pressure on ARM hardware.

---

### Latency Analysis

The latency characteristics reveal interesting differences in how each system handles queueing and contention:

| Percentile | Memcached | Memcrs | Ratio |
|-----------|-----------|--------|-------|
| p50 (median) | 0.487 ms | 1.703 ms | 3.5× |
| p99 | 4.543 ms | 4.959 ms | 1.1× |
| p99.9 | 6.047 ms | 13.119 ms | 2.2× |

**Key Observation**: The gap widens dramatically at tail percentiles. While median latencies show a 3.5× difference, the 99.9th percentile shows only a 2.2× difference, suggesting that Memcrs may have different contention characteristics at extreme load levels.

### Workload-Specific Performance

```text
Write-Heavy Impact:
  Memcached: Slight increase to 1.14 ms avg (from 1.12 ms)
  Memcrs: Significant increase to 2.07 ms avg (from 1.85 ms)
  Impact difference: 1.8% vs 11.8% relative increase
```

---

## Configuration Notes

### Single-Threaded vs Multi-Threaded Memory Configuration

**Single-Threaded Version** (Initial Benchmark):

- Memcrs memory: **50k**
- Memcached memory: **1024 MB**

**Multi-Threaded Version** (Updated Benchmark):

- Memcrs memory: **50 milions**
- Shows impact of runtime configuration on ARM multi-core systems

### Key Architectural Insights

**Single-Threaded Runtime (`current-thread`):**

- Each Tokio runtime should process independent requests in parallel
- Cannot utilize ARM multi-core capabilities effectively
- Results in performance bottleneck

**Multi-Threaded Runtime (`multi-thread`):**

- Work-stealing scheduler distributes tasks across worker threads
- Parallel processing leverages all available cores
- Better latency distribution due to reduced task queue contention
- Addresses the ARM multi-core utilization problem

The dramatic improvements with multi-threaded runtime (56% throughput gain, 2.7× tail latency improvement) validate the importance of proper async runtime selection on multi-core ARM systems.

---

**Test Configuration Changes**:

- Memory: Increased maximum capacity from 1 milion to 50 milion for memcrs_multi(-m 50M) to eliminate evictions
- Thread workers: 4 (half of available cores - can be tuned to match CPU core count)
- Tokio runtime: Multi-threaded work-stealing scheduler

**Variables Identified for Future Optimization**:

- Thread pool size tuning relative to ARM core configuration
- Memory allocation strategies
- Moka eviction policy optimization for ARM caches
- Connection buffer sizing for embedded systems

---

## Test Methodology

**Test Tool**: memtier_benchmark  
**Test Duration**: 5 runs per configuration per workload  
**Metrics Aggregated**: Best run, worst run, and averaged results  
**Date**: February 15, 2026  
**Reproducibility**: Identical hardware and software versions used throughout
