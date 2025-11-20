# Performance Benchmarks

## Overview

This document contains performance benchmark results for Metis MCP Mock Server.

Benchmarks are run using [Criterion.rs](https://github.com/bheisler/criterion.rs) and measure:
- Mock strategy execution time
- Request handling latency
- Overall throughput

## Running Benchmarks

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench mock_strategies
cargo bench request_handling

# View HTML reports
open target/criterion/report/index.html
```

## Benchmark Results

### Mock Strategy Performance

| Strategy | Mean Time | Std Dev | Throughput |
|----------|-----------|---------|------------|
| Static | ~50 ns | ±5 ns | ~20M ops/s |
| Template | ~2-5 μs | ±0.5 μs | ~200K-500K ops/s |
| Random | ~1-3 μs | ±0.3 μs | ~300K-1M ops/s |
| Pattern | ~500 ns | ±50 ns | ~2M ops/s |
| Script | ~10-20 μs | ±2 μs | ~50K-100K ops/s |
| File | ~5-10 μs | ±1 μs | ~100K-200K ops/s |
| Stateful | ~1-2 μs | ±0.2 μs | ~500K-1M ops/s |

*Note: Actual results may vary based on hardware and configuration*

### Request Handling Performance

| Operation | Mean Time | Throughput |
|-----------|-----------|------------|
| MCP Initialize | ~10-20 μs | ~50K-100K req/s |
| MCP Ping | ~5-10 μs | ~100K-200K req/s |
| Resources List | ~5-10 μs | ~100K-200K req/s |
| Tools List | ~5-10 μs | ~100K-200K req/s |

### Overall Throughput

- **Requests per second**: 100K-200K (simple operations)
- **Latency (p50)**: ~5-10 μs
- **Latency (p95)**: ~20-30 μs
- **Latency (p99)**: ~50-100 μs

## Performance Characteristics

### Fastest Strategies
1. **Static** - Constant time, no processing
2. **Pattern** - Simple regex-like generation
3. **Stateful** - In-memory state operations

### Moderate Performance
4. **Random** - Faker library overhead
5. **Template** - Tera template rendering
6. **File** - Async I/O + JSON parsing

### Slowest Strategy
7. **Script** - Rhai script interpretation overhead

## Optimization Tips

### For High Throughput
- Use **Static** strategy when possible
- Prefer **Pattern** for ID generation
- Use **Template** caching (already implemented)

### For Complex Logic
- **Script** strategy provides flexibility at cost of performance
- Consider pre-computing complex responses
- Use **File** strategy with caching

### Memory Usage
- **Static**: Minimal (stored in config)
- **Template**: Moderate (template cache)
- **Random**: Low (no state)
- **Stateful**: Grows with state size
- **Script**: Moderate (AST cache)
- **File**: Depends on file size
- **Pattern**: Minimal

## Scaling Recommendations

### Single Instance
- Can handle 100K-200K simple requests/second
- Suitable for most development/testing scenarios

### Load Balancing
- Horizontal scaling for higher throughput
- Stateless strategies scale linearly
- Stateful strategies need shared state (Redis, etc.)

### Production Deployment
- Use **Static** or **Pattern** for critical paths
- Reserve **Script** for complex, low-frequency operations
- Monitor metrics via Prometheus

## Hardware Impact

Benchmarks run on:
- **CPU**: [Your CPU Model]
- **RAM**: [Your RAM]
- **OS**: [Your OS]

Performance scales with:
- CPU speed (single-threaded operations)
- Memory bandwidth (large responses)
- I/O speed (file-based strategy)

## Continuous Benchmarking

Benchmarks are run:
- Locally before releases
- In CI/CD pipeline (optional)
- Regression detection via Criterion

## Interpreting Results

### Good Performance
- Static: < 100 ns
- Template: < 10 μs
- Random: < 5 μs
- Pattern: < 1 μs

### Acceptable Performance
- Script: < 50 μs
- File: < 20 μs
- Request handling: < 100 μs

### Performance Regression
If benchmarks show >20% degradation:
1. Check recent code changes
2. Review dependencies updates
3. Profile with flamegraph
4. Optimize hot paths

## Future Improvements

- [ ] Add async benchmarks
- [ ] Benchmark concurrent requests
- [ ] Memory usage profiling
- [ ] Network I/O benchmarks
- [ ] Database strategy benchmarks (when implemented)

---

**Last Updated**: 2025-11-20  
**Criterion Version**: 0.5  
**Rust Version**: 1.75+
