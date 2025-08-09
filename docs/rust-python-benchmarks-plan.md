# Rust vs Python API Performance Benchmark Plan

## Executive Summary

This document outlines a comprehensive plan to perform apples-to-apples performance comparisons between the Python (FastAPI) and Rust (Axum) implementations of the Blackledger API. The benchmarks will focus on three critical operations:

1. **Transaction Posting** - Single transaction posting latency and throughput
2. **Transaction Search** - Query performance with various filter combinations
3. **Balance Calculations** - Account balance retrieval and aggregation performance

## Objectives

- Establish baseline performance metrics for both implementations
- Ensure fair comparison with identical workloads and conditions
- Identify performance bottlenecks and optimization opportunities
- Validate the expected 2-3x performance improvement of Rust over Python
- Generate reproducible benchmark results for documentation

## Test Environment Setup

### Infrastructure Requirements

```yaml
Database:
  - PostgreSQL 16 
  - Dedicated test database for each implementation
  - Identical schema and indexes
  - Pre-populated with consistent test data
  - Connection pool sizes: 20 connections (same for both)

Application Servers:
  - Python: FastAPI with uvicorn (4 workers)
  - Rust: Axum with tokio runtime
  - Both running on same hardware
  - No authentication enabled (AUTH_ENABLED=false)
  - Identical logging levels (WARN)

Load Testing Tool:
  - Apache Bench (ab) or wrk for simple endpoints
  - k6 or Gatling for complex scenarios
  - Custom benchmark scripts for precise measurements
```

### Hardware Specifications

```yaml
Recommended Setup:
  CPU: 8+ cores (ARM64 or x86_64)
  RAM: 16GB minimum
  Storage: NVMe SSD
  Network: Localhost only (eliminate network variability)
  
Alternative Cloud Setup:
  - AWS EC2 m6i.2xlarge or equivalent
  - RDS PostgreSQL for consistent database performance
  - Both APIs on identical instance types
```

## Test Data Generation

### Data Volume Requirements

```yaml
Base Dataset:
  Ledgers: 10
  Accounts per Ledger: 1,000 (10,000 total)
  Currencies: 10 (USD, EUR, GBP, JPY, CHF, CAD, AUD, CNY, INR, BRL)
  Existing Transactions: 100,000
  Entries per Transaction: 2-10 (average 4)
  Total Entries: ~400,000

Account Structure:
  - Hierarchical with 3-4 levels
  - Mix of asset, liability, equity, revenue, expense accounts
  - Balanced account tree per ledger
```

### Data Generation Script

```python
# scripts/generate_benchmark_data.py
import uuid
import random
from datetime import datetime, timedelta
from decimal import Decimal

def generate_benchmark_data():
    """Generate consistent test data for benchmarking"""
    
    # Generate ledgers
    ledgers = []
    for i in range(10):
        ledgers.append({
            'id': str(uuid.uuid4()),
            'name': f'Benchmark Ledger {i}',
            'created': datetime.utcnow()
        })
    
    # Generate accounts hierarchy
    accounts = []
    for ledger in ledgers:
        # Root accounts
        assets = create_account(ledger['id'], 'Assets', 'DR')
        liabilities = create_account(ledger['id'], 'Liabilities', 'CR')
        equity = create_account(ledger['id'], 'Equity', 'CR')
        revenue = create_account(ledger['id'], 'Revenue', 'CR')
        expenses = create_account(ledger['id'], 'Expenses', 'DR')
        
        # Sub-accounts (100 per root)
        for root in [assets, liabilities, equity, revenue, expenses]:
            for i in range(200):
                create_sub_account(ledger['id'], root['id'], f'{root["name"]}-{i}')
    
    # Generate historical transactions
    transactions = []
    for i in range(100000):
        tx = generate_transaction(ledger_id, accounts, currencies)
        transactions.append(tx)
    
    return ledgers, accounts, transactions
```

## Benchmark Scenarios

### 1. Transaction Posting Performance

#### 1.1 Single Transaction Posting

```yaml
Test Case: Post individual transactions sequentially
Metrics:
  - Response time (p50, p95, p99)
  - Throughput (transactions/second)
  - CPU usage
  - Memory usage
  - Database connection pool utilization

Workload:
  - 1,000 transactions posted sequentially
  - Each transaction has 4 entries (2 debits, 2 credits)
  - Random accounts from same ledger
  - Random amounts between $1 and $10,000
  - Single currency per transaction

Expected Results:
  Python: ~50ms per transaction
  Rust: <20ms per transaction
```

#### 1.2 Concurrent Transaction Posting

```yaml
Test Case: Post transactions with multiple concurrent clients
Metrics:
  - Throughput under load
  - Response time degradation
  - Error rates
  - Optimistic locking conflicts

Workload:
  - 10, 50, 100 concurrent clients
  - Each client posts 100 transactions
  - Monitor version conflicts and retries
  - Measure successful vs failed transactions

Expected Results:
  Python: 100-200 TPS sustained
  Rust: 500-1000 TPS sustained
```

#### 1.3 Large Transaction Posting

```yaml
Test Case: Post complex transactions with many entries
Metrics:
  - Processing time vs entry count
  - Validation overhead
  - Memory usage spikes

Workload:
  - Transactions with 10, 50, 100, 500 entries
  - Multi-currency transactions
  - Complex validation scenarios

Expected Results:
  Linear scaling with entry count
  Rust 2-3x faster across all sizes
```

### 2. Transaction Search Performance

#### 2.1 Basic Search Queries

```yaml
Test Case: Search transactions with single filter
Metrics:
  - Query response time
  - Result set size impact
  - Index utilization

Queries:
  1. By ledger_id (returns ~10,000 transactions)
  2. By account_id (returns ~100 transactions)
  3. By currency (returns ~10,000 transactions)
  4. By date range (returns variable)
  5. By memo pattern (regex search)

Expected Results:
  Python: 30-100ms depending on result size
  Rust: 10-30ms depending on result size
```

#### 2.2 Complex Search Queries

```yaml
Test Case: Search with multiple filters and pagination
Metrics:
  - Complex query performance
  - Pagination overhead
  - Sort performance

Queries:
  1. Multi-field filter (ledger + account + currency)
  2. Date range + amount range + currency
  3. Regex pattern matching on memo
  4. Comma-delimited ID lists (IN queries)
  5. Pagination with offset/limit
  6. Sorting by different columns

Expected Results:
  Python: 50-200ms
  Rust: 15-60ms
```

#### 2.3 Search Under Load

```yaml
Test Case: Concurrent search requests
Metrics:
  - Query throughput
  - Response time under load
  - Database connection saturation

Workload:
  - 100 concurrent search requests
  - Mix of simple and complex queries
  - Random pagination offsets
  - Monitor connection pool usage

Expected Results:
  Python: 200-500 QPS
  Rust: 1000-2000 QPS
```

### 3. Balance Calculation Performance

#### 3.1 Single Account Balance

```yaml
Test Case: Get balance for individual accounts
Metrics:
  - Response time
  - Calculation accuracy
  - Cache effectiveness (if any)

Queries:
  1. Account with few entries (<100)
  2. Account with many entries (>10,000)
  3. Multi-currency account balances
  4. Point-in-time balance (as of date)

Expected Results:
  Python: 20-100ms
  Rust: 5-30ms
```

#### 3.2 Bulk Balance Retrieval

```yaml
Test Case: Get balances for multiple accounts
Metrics:
  - Batch query performance
  - Memory usage with large result sets
  - Streaming vs buffering

Queries:
  1. All accounts in a ledger (1,000 accounts)
  2. Filtered account list (100 accounts)
  3. Trial balance (all accounts with non-zero balance)
  4. Multi-currency balance aggregation

Expected Results:
  Python: 200-500ms
  Rust: 50-150ms
```

#### 3.3 Balance Calculation Under Load

```yaml
Test Case: Concurrent balance requests during transaction posting
Metrics:
  - Read performance during writes
  - Consistency under concurrent updates
  - Lock contention impact

Workload:
  - 50 clients reading balances
  - 10 clients posting transactions
  - Monitor response time variance
  - Verify balance consistency

Expected Results:
  Python: Significant degradation under load
  Rust: Minimal degradation, better concurrency
```

## Benchmark Implementation

### Directory Structure

```
benchmarks/
├── config/
│   ├── python.yaml      # Python API configuration
│   ├── rust.yaml        # Rust API configuration
│   └── database.yaml    # Database connection settings
├── data/
│   ├── generate.py      # Test data generation
│   ├── seed.sql         # Database seed script
│   └── fixtures/        # Pre-generated test data
├── scripts/
│   ├── run_all.sh       # Execute all benchmarks
│   ├── python/
│   │   ├── transaction_posting.py
│   │   ├── transaction_search.py
│   │   └── balance_calculation.py
│   └── rust/
│       ├── transaction_posting.rs
│       ├── transaction_search.rs
│       └── balance_calculation.rs
├── results/
│   ├── raw/             # Raw benchmark output
│   ├── processed/       # Processed results (JSON/CSV)
│   └── reports/         # Generated reports and graphs
└── tools/
    ├── analyze.py       # Result analysis and reporting
    ├── compare.py       # Side-by-side comparison
    └── visualize.py     # Generate graphs and charts
```

### Benchmark Execution Script

```bash
#!/bin/bash
# benchmarks/scripts/run_all.sh

set -e

# Configuration
PYTHON_URL="http://localhost:8000"
RUST_URL="http://localhost:8080"
ITERATIONS=1000
CONCURRENCY=10

echo "=== Blackledger Performance Benchmarks ==="
echo "Python API: $PYTHON_URL"
echo "Rust API: $RUST_URL"
echo ""

# Ensure both servers are running
check_health() {
    curl -f "$1/health" > /dev/null 2>&1
    return $?
}

if ! check_health "$PYTHON_URL"; then
    echo "Python API not responding at $PYTHON_URL"
    exit 1
fi

if ! check_health "$RUST_URL"; then
    echo "Rust API not responding at $RUST_URL"
    exit 1
fi

# Reset and seed database
echo "Preparing test data..."
python benchmarks/data/generate.py
psql $DATABASE_URL < benchmarks/data/seed.sql

# Run transaction posting benchmarks
echo ""
echo "=== Transaction Posting Benchmarks ==="
python benchmarks/scripts/python/transaction_posting.py \
    --url "$PYTHON_URL" \
    --iterations $ITERATIONS \
    --output benchmarks/results/raw/python_posting.json

cargo run --release --bin transaction_posting -- \
    --url "$RUST_URL" \
    --iterations $ITERATIONS \
    --output benchmarks/results/raw/rust_posting.json

# Run search benchmarks
echo ""
echo "=== Transaction Search Benchmarks ==="
python benchmarks/scripts/python/transaction_search.py \
    --url "$PYTHON_URL" \
    --queries benchmarks/data/search_queries.json \
    --output benchmarks/results/raw/python_search.json

cargo run --release --bin transaction_search -- \
    --url "$RUST_URL" \
    --queries benchmarks/data/search_queries.json \
    --output benchmarks/results/raw/rust_search.json

# Run balance calculation benchmarks
echo ""
echo "=== Balance Calculation Benchmarks ==="
python benchmarks/scripts/python/balance_calculation.py \
    --url "$PYTHON_URL" \
    --accounts benchmarks/data/account_list.json \
    --output benchmarks/results/raw/python_balance.json

cargo run --release --bin balance_calculation -- \
    --url "$RUST_URL" \
    --accounts benchmarks/data/account_list.json \
    --output benchmarks/results/raw/rust_balance.json

# Analyze and compare results
echo ""
echo "=== Analysis and Reporting ==="
python benchmarks/tools/analyze.py \
    --python-dir benchmarks/results/raw/ \
    --rust-dir benchmarks/results/raw/ \
    --output benchmarks/results/processed/

python benchmarks/tools/compare.py \
    --input benchmarks/results/processed/ \
    --output benchmarks/results/reports/comparison.md

python benchmarks/tools/visualize.py \
    --input benchmarks/results/processed/ \
    --output benchmarks/results/reports/

echo ""
echo "Benchmark complete! Results available in benchmarks/results/reports/"
```

## Measurement Methodology

### Key Metrics

```yaml
Performance Metrics:
  - Response Time:
    - Minimum, Maximum, Mean, Median
    - Percentiles: p50, p90, p95, p99, p99.9
    - Standard deviation
  
  - Throughput:
    - Requests per second (RPS)
    - Transactions per second (TPS)
    - Queries per second (QPS)
  
  - Resource Usage:
    - CPU utilization (per core and total)
    - Memory usage (RSS, heap, stack)
    - Database connections (active, idle, waiting)
    - Disk I/O (reads/writes per second)
  
  - Error Rates:
    - Success rate (2xx responses)
    - Client errors (4xx responses)
    - Server errors (5xx responses)
    - Timeout rate

Consistency Metrics:
  - Response time variance over time
  - Performance degradation under load
  - Warmup time to stable performance
  - Recovery time after load spike
```

### Statistical Considerations

```yaml
Sample Size:
  - Minimum 1,000 iterations per test
  - Discard first 100 iterations (warmup)
  - Run each test 3 times, report median

Variance Control:
  - Isolate test environment
  - Disable unnecessary services
  - Fixed CPU frequency (no throttling)
  - Consistent database state between runs
  - Monitor for external interference

Statistical Analysis:
  - Calculate confidence intervals (95%)
  - Identify and remove outliers (>3σ)
  - Test for statistical significance
  - Report coefficient of variation
```

## Reporting Format

### Summary Report Template

```markdown
# Blackledger Performance Benchmark Results

## Executive Summary
- Date: [YYYY-MM-DD]
- Python Version: [FastAPI version, Python version]
- Rust Version: [Axum version, Rust version]
- Database: PostgreSQL [version]
- Test Duration: [total time]

## Overall Performance Comparison

| Metric                       | Python  | Rust   | Improvement |
|------------------------------|---------|--------|-------------|
| Transaction Posting (p50)    | XX ms   | XX ms  | X.Xx        |
| Transaction Search (p50)     | XX ms   | XX ms  | X.Xx        |
| Balance Calculation (p50)    | XX ms   | XX ms  | X.Xx        |
| Max Throughput (TPS)         | XXX     | XXXX   | X.Xx        |
| Memory Usage (avg)           | XXX MB  | XX MB  | X.Xx        |
| CPU Usage (avg)              | XX%     | XX%    | X.Xx        |

## Detailed Results

### Transaction Posting
[Detailed metrics, graphs, analysis]

### Transaction Search
[Detailed metrics, graphs, analysis]

### Balance Calculation
[Detailed metrics, graphs, analysis]

## Conclusions
[Key findings and recommendations]
```

### Visualization Requirements

```yaml
Graphs to Generate:
  1. Response Time Distribution:
     - Histogram of response times
     - CDF (Cumulative Distribution Function)
     - Box plots for percentiles
  
  2. Throughput Over Time:
     - Line graph showing RPS over test duration
     - Identify performance plateaus
     - Show degradation points
  
  3. Latency vs Throughput:
     - Scatter plot showing relationship
     - Identify optimal operating point
     - Show saturation curve
  
  4. Resource Usage:
     - Time series of CPU and memory
     - Correlation with load
     - Efficiency metrics (RPS per CPU%)
  
  5. Comparison Charts:
     - Side-by-side bar charts
     - Speedup ratios
     - Relative performance indices
```

## Implementation Timeline

### Phase 1: Environment Setup (2 days)
- [ ] Configure test servers
- [ ] Set up monitoring tools
- [ ] Install load testing tools
- [ ] Create Docker compose for consistent environment

### Phase 2: Data Generation (2 days)
- [ ] Implement data generation scripts
- [ ] Create seed database
- [ ] Validate data consistency
- [ ] Generate query patterns

### Phase 3: Benchmark Scripts (3 days)
- [ ] Implement Python benchmark clients
- [ ] Implement Rust benchmark clients
- [ ] Create execution orchestration
- [ ] Test script reliability

### Phase 4: Execution (2 days)
- [ ] Run warmup tests
- [ ] Execute full benchmark suite
- [ ] Monitor for anomalies
- [ ] Collect raw results

### Phase 5: Analysis and Reporting (2 days)
- [ ] Process raw results
- [ ] Generate statistical analysis
- [ ] Create visualizations
- [ ] Write final report

## Success Criteria

```yaml
Minimum Requirements:
  - Rust shows >2x improvement in transaction posting
  - Rust shows >2x improvement in search queries
  - Rust shows >2x improvement in balance calculations
  - No regression in functionality
  - Results are reproducible (±5% variance)

Stretch Goals:
  - 3x improvement in concurrent scenarios
  - 5x improvement in complex queries
  - 10x improvement in maximum throughput
  - 50% reduction in memory usage
  - Sub-10ms p99 latency for simple operations
```

## Risk Mitigation

```yaml
Potential Issues:
  1. Database bottleneck masking API differences
     Mitigation: Use connection pooling, optimize queries
  
  2. Test data not representative
     Mitigation: Model after real-world patterns
  
  3. Network variability affecting results
     Mitigation: Run on localhost, use Unix sockets
  
  4. Caching skewing results
     Mitigation: Clear caches between runs
  
  5. JIT compilation affecting Python
     Mitigation: Proper warmup period
  
  6. Rust debug vs release builds
     Mitigation: Always use --release for Rust
```

## Next Steps

1. Review and approve benchmark plan
2. Set up dedicated test environment
3. Begin implementation of benchmark scripts
4. Schedule benchmark execution window
5. Prepare stakeholder communication

## Appendix

### A. Sample Benchmark Output

```json
{
  "test": "transaction_posting",
  "implementation": "rust",
  "timestamp": "2024-01-15T10:30:00Z",
  "configuration": {
    "iterations": 1000,
    "concurrency": 1,
    "entries_per_transaction": 4
  },
  "results": {
    "response_times": {
      "min": 12.3,
      "max": 45.6,
      "mean": 18.7,
      "median": 17.2,
      "p90": 22.1,
      "p95": 24.3,
      "p99": 31.2,
      "std_dev": 4.2
    },
    "throughput": {
      "average_rps": 53.5,
      "peak_rps": 81.2
    },
    "errors": {
      "total": 0,
      "rate": 0.0
    }
  }
}
```

### B. References

- [Apache Bench Documentation](https://httpd.apache.org/docs/2.4/programs/ab.html)
- [k6 Load Testing Tool](https://k6.io/)
- [Criterion.rs Benchmarking](https://github.com/bheisler/criterion.rs)
- [PostgreSQL Performance Tuning](https://wiki.postgresql.org/wiki/Performance_Optimization)

---

*Document Version: 1.0*
*Created: 2025-08-09*
*Author: Blackledger Performance Team*
