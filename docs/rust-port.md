# Blackledger Rust Port Implementation Plan

## Overview

This document outlines the implementation plan for porting the Blackledger API from Python/FastAPI to Rust/Axum while maintaining full API compatibility and database schema integrity.

## Technology Stack

### Core Dependencies
- **axum** (0.7+): Async web framework
- **sqlx** (0.8+): Compile-time checked SQL with PostgreSQL support
- **tokio** (1.0+): Async runtime
- **serde** (1.0+): Serialization/deserialization
- **serde_json**: JSON support
- **rust_decimal** (1.36+): Decimal arithmetic for financial precision
- **chrono** (0.4+): DateTime handling with timezone support
- **uuid** (1.0+): UUID generation and parsing
- **thiserror** (1.0+): Error type derivation
- **anyhow**: Error handling in application code

### Authentication
- **jsonwebtoken** (9.0+): JWT token validation
- **reqwest** (0.12+): HTTP client for JWKS retrieval
- **tower** (0.5+): Middleware stack

### Development & Testing
- **tracing** (0.1+): Structured logging
- **tracing-subscriber**: Log formatting
- **dotenv**: Environment variable loading
- **cargo-watch**: Auto-reload during development
- **sqlx-cli**: Database migrations and testing

## Project Structure

```
backend/
├── Cargo.toml
├── .envrc.example
├── migrations/             # SQL migrations (from Python version)
│   └── *.sql
├── src/
│   ├── main.rs            # Application entry point
│   ├── lib.rs             # Library root
│   ├── config.rs          # Configuration management
│   ├── error.rs           # Custom error types
│   ├── models/            # Domain models
│   │   ├── mod.rs
│   │   ├── currency.rs
│   │   ├── ledger.rs
│   │   ├── account.rs
│   │   ├── transaction.rs
│   │   └── entry.rs
│   ├── db/                # Database layer
│   │   ├── mod.rs
│   │   ├── pool.rs        # Connection pool management
│   │   ├── queries/       # SQL query modules
│   │   └── migrations.rs  # Migration runner
│   ├── api/               # HTTP API layer
│   │   ├── mod.rs
│   │   ├── router.rs      # Route definitions
│   │   ├── middleware/    # Custom middleware
│   │   ├── handlers/      # Request handlers
│   │   │   ├── currencies.rs
│   │   │   ├── ledgers.rs
│   │   │   ├── accounts.rs
│   │   │   └── transactions.rs
│   │   └── extractors.rs  # Custom Axum extractors
│   ├── services/          # Business logic
│   │   ├── mod.rs
│   │   ├── validation.rs  # Transaction validation
│   │   ├── posting.rs     # Transaction posting logic
│   │   └── search.rs      # Complex search queries
│   └── utils/             # Utilities
│       ├── mod.rs
│       ├── decimal.rs     # Decimal helpers
│       └── datetime.rs    # DateTime helpers
└── tests/
    ├── common/            # Test fixtures and helpers
    └── api/               # Integration tests

```

## Implementation Phases

### Phase 1: Foundation (Week 1-2)
- [ ] Project setup with Cargo workspace
- [ ] Basic Axum server with /api endpoint (health check)
- [ ] SQLx integration with connection pooling
- [ ] Configuration management (environment variables)
- [ ] Error handling framework with custom exception handlers
- [ ] Basic logging setup
- [ ] Docker Compose setup for development

**Testing**:
- [ ] Unit tests for configuration loading
- [ ] Unit tests for error type conversions
- [ ] Integration test for /api endpoint
- [ ] Integration test for database connectivity
- [ ] Test database setup/teardown fixtures

**Deliverables**: Basic server matching Python foundation

### Phase 2: Core Models & Database (Week 2-3)
- [ ] Define Rust structs for all domain models
- [ ] Implement serde serialization/deserialization
- [ ] Port SQL migrations to SQLx format
- [ ] Implement decimal precision handling
- [ ] Create database query functions
- [ ] Add compile-time SQL verification

**Testing**:
- [ ] Unit tests for model validation rules
- [ ] Unit tests for decimal arithmetic operations
- [ ] Unit tests for serde serialization/deserialization
- [ ] Unit tests for custom type conversions
- [ ] Integration tests for migration runner
- [ ] Integration tests for basic CRUD queries

**Deliverables**: All models defined with database CRUD operations and comprehensive test coverage

### Phase 3: Currency & Ledger Endpoints (Week 3-4)
**Currencies**
- [ ] POST /currencies - Create new currency
- [ ] GET /currencies - List all currencies

**Currency Testing**:
- [ ] Unit tests for currency code validation (ISO 4217)
- [ ] Integration tests for POST /currencies success cases
- [ ] Integration tests for POST /currencies error cases (duplicates, invalid codes)
- [ ] Integration tests for GET /currencies

**Ledgers**
- [ ] POST /ledgers - Create new ledger
- [ ] GET /ledgers - List all ledgers with pagination
- [ ] GET /ledgers/{id} - Get single ledger

**Ledger Testing**:
- [ ] Unit tests for ledger name validation
- [ ] Unit tests for pagination logic
- [ ] Integration tests for POST /ledgers
- [ ] Integration tests for GET /ledgers with pagination
- [ ] Integration tests for GET /ledgers/{id} (found and not found)
- [ ] Integration tests for ledger isolation (multi-tenancy)

**Deliverables**: Currency and Ledger endpoints with full test coverage

### Phase 4: Account & Transaction Endpoints (Week 4-5)
**Accounts**
- [ ] POST /accounts - Create account with parent hierarchy
- [ ] PATCH /accounts/{id} - Update account (name, metadata)
- [ ] GET /accounts - List accounts with filtering
- [ ] GET /accounts/{id} - Get single account with balance

**Account Testing**:
- [ ] Unit tests for account hierarchy validation
- [ ] Unit tests for normal balance (DR/CR) logic
- [ ] Unit tests for account number formatting
- [ ] Integration tests for POST /accounts with parent relationships
- [ ] Integration tests for PATCH /accounts/{id}
- [ ] Integration tests for GET /accounts with filters
- [ ] Integration tests for GET /accounts/{id} with balance calculation
- [ ] Integration tests for account versioning logic

**Transactions**
- [ ] POST /transactions - Create transaction with validation
  - [ ] Implement double-entry validation
  - [ ] Account versioning (optimistic locking)
  - [ ] Multi-currency balance checking
- [ ] GET /transactions - List transactions with search
- [ ] GET /transactions/{id} - Get single transaction
- [ ] GET /entries - List entries with filtering

**Transaction Testing**:
- [ ] Unit tests for double-entry validation logic
- [ ] Unit tests for multi-currency balance checking
- [ ] Unit tests for entry debit/credit validation
- [ ] Integration tests for POST /transactions success cases
- [ ] Integration tests for POST /transactions validation errors
- [ ] Integration tests for optimistic locking conflicts
- [ ] Integration tests for GET /transactions with search params
- [ ] Integration tests for GET /transactions/{id}
- [ ] Integration tests for GET /entries with filters
- [ ] Integration tests for transaction immutability constraints
- [ ] Concurrent transaction posting tests

**Deliverables**: Complete API with all model endpoints and exhaustive test coverage

### Phase 5: Search & Balance Features (Week 5-6)
- [ ] Complex search implementation matching Python patterns
- [ ] Balance calculation queries (/accounts/balances endpoint)
- [ ] Multi-currency validation
- [ ] Performance optimization
- [ ] Connection pooling tuning

**Testing**:
- [ ] Unit tests for search query builders
- [ ] Unit tests for balance calculation algorithms
- [ ] Integration tests for complex search scenarios
- [ ] Integration tests for multi-currency transactions
- [ ] Integration tests for balance endpoint
- [ ] Performance benchmarks for balance calculations
- [ ] Load tests for concurrent operations

**Deliverables**: Feature parity with Python implementation

### Phase 6: Authentication (Week 6-7)
- [ ] JWT token validation middleware
- [ ] JWKS (JSON Web Key Set) integration
- [ ] Bearer token extraction from Authorization header
- [ ] Configurable auth that can be disabled for testing
- [ ] Token expiration and claims validation

**Testing**:
- [ ] Unit tests for JWT token parsing and validation
- [ ] Unit tests for JWKS key retrieval
- [ ] Integration tests for authenticated endpoints
- [ ] Integration tests for unauthorized access
- [ ] Tests for disabled auth mode
- [ ] Security tests for token expiration handling

**Deliverables**: JWT authentication matching Python implementation

### Phase 7: Final Validation & Deployment Prep (Week 7-8)
- [ ] Test coverage report generation and gap analysis
- [ ] End-to-end test scenarios matching Python test suite
- [ ] Performance comparison benchmarks vs Python version
- [ ] Migration guide from Python version
- [ ] Deployment documentation
- [ ] Docker image creation and optimization

**Deliverables**: 90%+ verified test coverage, deployment-ready application

## API Compatibility Requirements

### Endpoint Structure
Maintain exact endpoint paths and HTTP methods as implemented in Python:
```
GET    /api                    # Health check endpoint
GET    /api/currencies         # Search currencies
POST   /api/currencies         # Create/update currency

GET    /api/ledgers            # Search ledgers
POST   /api/ledgers            # Create/update ledgerG

GET    /api/accounts           # Search accounts
POST   /api/accounts           # Create/update account
GET    /api/accounts/balances  # Get account balances

GET    /api/transactions       # Search transactions
POST   /api/transactions       # Create transaction
```

### JSON Schema Compatibility
- Preserve all field names exactly
- Maintain same date/time formats (ISO 8601)
- Keep decimal precision (no floating point)
- Support same query parameters
- Return identical error response structure

### Database Schema
- No changes to existing table structure
- Maintain all constraints and triggers
- Preserve column names and types
- Keep same index definitions

## Critical Implementation Details

### 1. Decimal Handling
```rust
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Entry {
    #[serde(with = "rust_decimal::serde::str")]
    dr: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::str")]
    cr: Option<Decimal>,
}
```

### 2. Transaction Validation
- Ensure sum of debits equals sum of credits per currency
- Validate all accounts exist before posting
- Check account versions for optimistic locking
- Atomic database transactions for posting

### 3. Immutability Enforcement
- Rely on existing database triggers
- Never generate UPDATE/DELETE queries for transactions/entries
- Implement corrections via offsetting transactions

### 4. Error Handling Pattern
```rust
#[derive(thiserror::Error, Debug)]
enum ApiError {
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Transaction does not balance")]
    UnbalancedTransaction,
    
    #[error("Account version mismatch")]
    OptimisticLockError,
    
    #[error("Database error")]
    Database(#[from] sqlx::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        // Map to appropriate HTTP status codes
    }
}
```

### 5. Database Connection Pool
```rust
let pool = PgPoolOptions::new()
    .max_connections(20)
    .min_connections(5)
    .acquire_timeout(Duration::from_secs(3))
    .idle_timeout(Duration::from_secs(600))
    .connect(&database_url)
    .await?;
```

## Testing Strategy

### Unit Tests
- Model validation logic
- Decimal arithmetic operations
- Date/time handling
- Error type conversions

### Integration Tests
- Each API endpoint with various inputs
- Database constraint violations
- Transaction rollback scenarios
- Concurrent modification handling

### Test Database Management
```rust
#[sqlx::test]
async fn test_post_transaction(pool: PgPool) {
    // SQLx automatically handles test transactions
    // Each test runs in isolation with rollback
}
```

### Performance Testing
- Benchmark transaction posting
- Measure balance calculation queries
- Test concurrent request handling
- Profile memory usage under load

## Migration Checklist

- [ ] All Python tests passing in Rust
- [ ] API responses byte-for-byte compatible
- [ ] Performance metrics equal or better
- [ ] Database migrations applied successfully
- [ ] Documentation updated
- [ ] Deployment scripts adapted
- [ ] Monitoring/logging configured
- [ ] Load testing completed
- [ ] Rollback procedure documented
- [ ] Team training completed

## Risk Mitigation

### Potential Challenges
1. **Decimal precision differences**: Extensive testing with edge cases
2. **Async/await complexity**: Use tokio::test for async tests
3. **SQL query differences**: Validate all queries with SQLx compile-time checking
4. **JWT library differences**: Ensure token validation is identical
5. **Date/time timezone handling**: Use chrono with explicit timezone handling

### Rollback Strategy
- Maintain Python version in parallel during transition
- Use feature flags to route traffic
- Database schema remains unchanged
- Quick switch back via load balancer/proxy

## Success Criteria

- [ ] 100% API endpoint compatibility
- [ ] Zero data corruption or loss
- [ ] Response time ≤ Python version
- [ ] Memory usage ≤ Python version  
- [ ] 90%+ test coverage
- [ ] Zero critical security vulnerabilities
- [ ] Successful production deployment
- [ ] 30-day stable operation post-deployment

## Next Steps

1. Set up Rust project structure
2. Implement Phase 1 foundation
3. Create initial database models
4. Set up CI/CD pipeline
5. Begin incremental migration