#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Running tests with code coverage...${NC}"

# Ensure database migrations are up to date
echo -e "${YELLOW}Running database migrations...${NC}"
sqlx migrate run

# Run cargo-tarpaulin with various options
# --ignore-tests: Don't include test functions in coverage
# --out: Output formats (Html for HTML report, Stdout for terminal output)
# --engine: Use llvm engine for better accuracy
# --exclude-files: Exclude certain files from coverage
# --timeout: Test timeout in seconds
# --skip-clean: Don't clean build artifacts before running
echo -e "${YELLOW}Running cargo-tarpaulin...${NC}"
cargo tarpaulin \
    --ignore-tests \
    --out Html \
    --out Stdout \
    --engine llvm \
    --exclude-files "*/tests/*" \
    --exclude-files "*/examples/*" \
    --exclude-files "*/migrations/*" \
    --exclude-files "*/target/*" \
    --timeout 120 \
    --skip-clean \
    --target-dir target/tarpaulin

# Check if coverage report was generated
if [ -f "tarpaulin-report.html" ]; then
    echo -e "${GREEN}Coverage report generated: tarpaulin-report.html${NC}"
    
    # Extract coverage percentage from the output
    echo -e "${GREEN}Opening coverage report in browser...${NC}"
    if command -v open &> /dev/null; then
        open tarpaulin-report.html
    elif command -v xdg-open &> /dev/null; then
        xdg-open tarpaulin-report.html
    else
        echo -e "${YELLOW}Please open tarpaulin-report.html manually to view the coverage report${NC}"
    fi
else
    echo -e "${RED}Failed to generate coverage report${NC}"
    exit 1
fi