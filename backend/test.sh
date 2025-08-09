#!/bin/bash
set -e

# Parse command line arguments
COVERAGE=false
CLEAN=false
QUICK=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --coverage|-c)
            COVERAGE=true
            shift
            ;;
        --clean)
            CLEAN=true
            shift
            ;;
        --quick|-q)
            QUICK=true
            shift
            ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --coverage, -c  Run tests with coverage report (tarpaulin)"
            echo "  --clean         Clean all target directories before running"
            echo "  --quick, -q     Skip database setup (assumes it's already running)"
            echo "  --help, -h      Show this help message"
            echo ""
            echo "Examples:"
            echo "  $0              # Run tests without coverage"
            echo "  $0 --coverage   # Run tests with coverage report"
            echo "  $0 --quick      # Run tests quickly (skip DB setup)"
            echo "  $0 --clean -c   # Clean targets and run with coverage"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Clean target directories if requested
if [ "$CLEAN" = true ]; then
    echo "Cleaning target directories..."
    rm -rf target/debug
    rm -rf target/release
    rm -rf target/tarpaulin
    echo "Target directories cleaned."
fi

# Setup database (unless --quick is specified)
if [ "$QUICK" = false ]; then
    echo "Setting up test database..."
    
    # Recreate test database
    docker compose down -v postgres-test || true
    docker compose up -d postgres-test
    
    # Export test environment variables
    export DATABASE_URL="postgresql://blackledger_test:test@localhost:5434/blackledger_test"
    export TEST_DATABASE_URL="postgresql://blackledger_test:test@localhost:5434/blackledger_test"
    
    # Wait for test database to become available
    until psql -c "SELECT true;" $TEST_DATABASE_URL &>/dev/null; do
        echo "Waiting for test database to be available..."
        sleep 1
    done
    
    # Run migrations
    echo "Running migrations..."
    sqlx migrate run
else
    echo "Skipping database setup (--quick mode)..."
    export DATABASE_URL="postgresql://blackledger_test:test@localhost:5434/blackledger_test"
    export TEST_DATABASE_URL="postgresql://blackledger_test:test@localhost:5434/blackledger_test"
fi

# Run tests
if [ "$COVERAGE" = true ]; then
    echo "Running tests with coverage..."
    
    # Clean tarpaulin's specific target directory for fresh instrumentation
    # This ensures accurate coverage while preserving the skip-clean optimization
    if [ "$CLEAN" = false ]; then
        echo "Cleaning tarpaulin target for accurate coverage..."
        rm -rf target/tarpaulin
    fi
    
    # Run with tarpaulin using config file (which has skip-clean=true)
    # The --out Html is specified here to ensure HTML output
    cargo tarpaulin --config tarpaulin.toml --skip-clean --out Html --all-features -- --test-threads=1
    
    echo ""
    echo "Coverage report generated:"
    echo "  - HTML: tarpaulin-report.html"
    echo "  - LCOV: lcov.info"
    echo "  - JSON: tarpaulin-report.json"
else
    echo "Running tests..."
    # Run tests sequentially to avoid database state conflicts
    RUST_BACKTRACE=1 cargo test --all-features -- --test-threads=1
fi

echo ""
echo "Tests completed successfully!"