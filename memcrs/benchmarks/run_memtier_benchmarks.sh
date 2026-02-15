#!/bin/bash

# Memcached Performance Benchmarking Script
# Executes read-heavy, write-heavy, and stress tests
# Usage: ./run_memcached_benchmarks.sh <test_name>

set -e


# Option parsing for --native
NATIVE=0
POSITIONAL=()
while [[ $# -gt 0 ]]; do
    key="$1"
    case $key in
        --native)
        NATIVE=1
        shift # past argument
        ;;
        *)
        POSITIONAL+=("$1") # save positional arg
        shift # past argument
        ;;
    esac
done
set -- "${POSITIONAL[@]}"

# Check for parameter
if [ $# -ne 1 ]; then
    echo "Usage: $0 <test_name> [--native]"
    echo "Example: $0 my_benchmark --native"
    exit 1
fi

TEST_NAME="$1"
NUM_RUNS=5
THREADS=4
PROTOCOL="memcache_binary"
PORT=11211
TEST_DURATION=60

echo "=========================================="
echo "Memcached Benchmark Suite"
echo "=========================================="
echo "Test Name: $TEST_NAME"
echo "Number of Runs: $NUM_RUNS"
echo "Threads: $THREADS"
echo "Protocol: $PROTOCOL"
echo "Test Duration: ${TEST_DURATION}s"
echo "=========================================="
echo ""
echo "Note: Ensure memcached is running on localhost:$PORT before executing this script."
echo ""

# Function to run a benchmark test
run_benchmark() {
    local test_type=$1
    local ratio=$2
    local data_size=$3
    local key_maximum=$4
    local key_pattern=$5
    local description=$6

    echo "Running $description tests..."
    echo "Configuration: ratio=$ratio, data_size=$data_size, keys=$key_maximum, pattern=$key_pattern"
    echo ""

    echo "[${test_type}] Run $run/$NUM_RUNS..."

    hdr_prefix="${test_type}_${TEST_NAME}_run_${run}"

    if [ "$NATIVE" -eq 1 ]; then
        memtier_benchmark \
            --port=$PORT \
            --run-count=$NUM_RUNS \
            --protocol=$PROTOCOL \
            --threads=$THREADS \
            --clients=50 \
            --test-time=$TEST_DURATION \
            --ratio=$ratio \
            --data-size=$data_size \
            --key-maximum=$key_maximum \
            --key-pattern=$key_pattern \
            --hdr-file-prefix="$hdr_prefix" \
            --hide-histogram
    else
        docker run --mount type=bind,src=.,dst=/mnt \
            --workdir /mnt \
            --net=host -it \
            --rm redislabs/memtier_benchmark:latest \
            --port=$PORT \
            --run-count=$NUM_RUNS \
            --protocol=$PROTOCOL \
            --threads=$THREADS \
            --clients=50 \
            --test-time=$TEST_DURATION \
            --ratio=$ratio \
            --data-size=$data_size \
            --key-maximum=$key_maximum \
            --key-pattern=$key_pattern \
            --hdr-file-prefix="$hdr_prefix" \
            --hide-histogram
    fi

    echo ""
    echo "Completed $description tests."
    echo ""
}

# Test 1: Read-Heavy Workload (90% reads, 10% writes)
echo "=== TEST 1: READ-HEAVY WORKLOAD ==="
run_benchmark "read_heavy" "1:10" "256" "1000000" "R:R" "Read-Heavy Workload"

# Test 2: Write-Heavy Workload (50/50 reads and writes)
echo "=== TEST 2: WRITE-HEAVY WORKLOAD ==="
run_benchmark "write_heavy" "5:5" "1024" "1000000" "R:R" "Write-Heavy Workload"

# Test 3: Stress Test (High throughput with Zipfian distribution)
echo "=== TEST 3: STRESS TEST ==="
run_benchmark "stress_test" "1:10" "512" "10000000" "G:G" "Stress Test"

echo "=========================================="
echo "All benchmarks completed!"
echo "=========================================="
echo ""
echo "Generated files:"
echo "  Read-Heavy:  read_heavy_${TEST_NAME}_run_*.{hgrm,txt}"
echo "  Write-Heavy: write_heavy_${TEST_NAME}_run_*.{hgrm,txt}"
echo "  Stress Test: stress_test_${TEST_NAME}_run_*.{hgrm,txt}"
echo ""
echo "To analyze results:"
echo "  python3 analyze_hgrm.py read_heavy_${TEST_NAME}_run_*.hgrm -o comparison.png"
echo "  python3 analyze_hgrm.py read_heavy_${TEST_NAME}_run_*.hgrm --stats-only"
echo ""
echo "Online visualization:"
echo "  1. Go to https://hdrhistogram.github.io/HdrHistogram/plotFiles.html"
echo "  2. Upload the .txt files to compare latency distributions."
echo ""
echo "Cleanup:"
echo "Removing generated files for test: $TEST_NAME"
echo "find . -name \"${TEST_NAME}_run_*.hgrm\" -delete"
echo "find . -name \"${TEST_NAME}_run_*.txt\" -delete"
echo ""
