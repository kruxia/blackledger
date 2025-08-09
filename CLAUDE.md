# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Blackledger is a double-entry accounting system implemented as a REST API. The current implementation uses FastAPI and PostgreSQL, but **the target architecture is to port this to Rust using Axum and SQLx**. Key principles:
- **Immutable transactions**: Once posted, transactions/entries cannot be modified or deleted
- **Double-entry accounting**: All transactions must balance (debits = credits) 
- **Multi-currency**: Each entry specifies its currency code
- **Decimal precision**: Uses Python Decimal for financial calculations

## Common Development Commands

```bash
# Run tests with coverage (creates test database, runs migrations, executes pytest)
./script/test.sh

# Run a single test file
pytest tests/test_api.py::TestClass::test_method -xvs

# Lint code (uses ruff)
./script/lint.sh

# Format code (uses ruff)
./script/fmt.sh

# Start local development environment (PostgreSQL, Keycloak, app)
docker compose up

# Run migrations on test database
python -m blackledger.migration --dburl postgresql://blackledger_test@localhost/blackledger_test
```

## Architecture Overview

### Core Models (blackledger/model.py)
- **Currency**: Validates currency codes
- **Ledger**: Top-level container for accounts and transactions
- **Account**: Chart of accounts with parent hierarchy and normal balance (DR/CR)
- **Transaction**: Contains multiple entries that must balance
- **Entry**: Individual debit or credit within a transaction

### API Layer (blackledger/api/)
- RESTful endpoints for all entities
- JWT authentication via Keycloak integration
- Comprehensive search/filtering capabilities
- Transaction posting with automatic validation

### Database Layer
- Custom query builder using sqly library
- PostgreSQL with psycopg connection pooling
- Database-level immutability constraints via triggers
- Migrations in blackledger/migration/data/*.yaml

### Key Architectural Patterns

1. **Immutability Enforcement**: Database triggers prevent UPDATE/DELETE on transactions and entries. Corrections require new offsetting transactions.

2. **Account Versioning**: Each account tracks its latest entry ID as a version. When posting transactions, you must provide the current version to prevent concurrent modifications.

3. **Balance Validation**: Transactions must balance across all currencies. The system validates sum(debits) = sum(credits) for each currency.

4. **Multi-Currency Handling**: Each entry specifies its currency. Balances are calculated per currency per account. No automatic conversion.

## Testing Requirements

- Minimum 90% test coverage required
- Tests use pytest with asyncio support
- Session-scoped fixtures for database setup
- Function-scoped fixtures for test isolation
- Test database is recreated for each test run

### Test Data Isolation (IMPORTANT)

**ALWAYS use UUIDs for names in test data to prevent conflicts between test runs.**

The test database is shared across test runs and some entities (like ledgers) have unique constraints on names. To prevent test failures due to duplicate data:

```rust
// ❌ BAD - Will fail on second test run
let ledger = create_ledger(&pool, &CreateLedger {
    name: "Test Ledger".to_string(),
}).await?;

// ✅ GOOD - Always succeeds
use uuid::Uuid;
let ledger = create_ledger(&pool, &CreateLedger {
    name: format!("Test Ledger {}", Uuid::new_v4()),
}).await?;
```

This applies to:
- Ledger names (have unique constraint)
- Any other test data that might conflict

For accounts and other entities without unique constraints on names, UUIDs are optional but recommended for clarity in test output.

## Important Constraints

1. **Transaction Posting**: When posting transactions, ensure:
   - Sum of debits equals sum of credits for each currency
   - Account versions match current state (optimistic locking)
   - All referenced accounts exist in the ledger

2. **Search Operations**: The API supports complex search parameters with filters for:
   - Pattern matching (regex) on text fields
   - Date ranges on posted/effective timestamps
   - Currency and amount filters

3. **Error Handling**: The system uses custom exceptions that map to appropriate HTTP status codes. Database constraint violations are caught and returned as meaningful error messages.

## Rust Port Target Architecture

The API is being ported to Rust with the following stack:
- **Web Framework**: Axum (async HTTP framework)
- **Database**: SQLx (compile-time checked SQL queries)
- **Serialization**: serde with serde_json
- **Decimal Handling**: rust_decimal for financial precision
- **Authentication**: JWT validation (use jsonwebtoken crate)
- **Error Handling**: thiserror for custom error types
- **Testing**: Built-in Rust testing with SQLx test transactions

When implementing in Rust:
- Maintain the same REST API endpoints and JSON structure for compatibility
- Use SQLx's compile-time query verification for type safety
- Implement the same immutability constraints and validation rules
- Use / create tower middleware for authentication and request tracing
- Ensure decimal precision is maintained with rust_decimal::Decimal

### Running Rust Tests

```bash
# Run all tests (requires DATABASE_URL to be set)
export DATABASE_URL="postgresql://blackledger_test:test@localhost:5434/blackledger_test"
cargo test

# Run specific test file
cargo test --test handlers_test

# Run with output for debugging
cargo test -- --nocapture

# Run tests with coverage (if using cargo-tarpaulin)
cargo tarpaulin --out Html
```

**Important**: Some tests use `#[sqlx::test]` which require DATABASE_URL. Others use the common test setup in `tests/common/mod.rs` which handles database connections internally.