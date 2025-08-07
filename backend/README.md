# Blackledger - Rust Backend

A high-performance, immutable double-entry accounting system built with Rust, Axum, and PostgreSQL.

## 🚀 Features

- **Immutable Transactions**: Once posted, transactions cannot be modified or deleted
- **Double-Entry Accounting**: Enforces balanced transactions (debits = credits)
- **Multi-Currency Support**: Handle multiple currencies in a single ledger
- **Optimistic Locking**: Prevent concurrent modifications with account versioning
- **Decimal Precision**: Financial-grade accuracy with `rust_decimal`
- **RESTful API**: Clean, consistent API with pagination and search
- **JWT Authentication**: Secure endpoints with token-based auth
- **Audit Trail**: Automatic tracking of who posted what and when
- **Unified Transaction Format**: Responses include entries for consistency with requests
- **Comprehensive Search**: Regex patterns, comma-delimited filters, and sorting

## Prerequisites

- Rust 1.88+
- Docker and Docker Compose
- PostgreSQL 16+ (for local development without Docker)

## Quick Start

1. Copy environment variables:
```bash
cp .env.example .env
```

2. Start the development environment:
```bash
docker compose up
```

The API will be available at http://localhost:8002/api

## Development

### Running locally

1. Install dependencies:
```bash
cargo build
```

2. Set up database:
```bash
# Start PostgreSQL
docker compose up postgres

# Run migrations
sqlx migrate run
```

3. Run the server:
```bash
cargo run
```

### Auto-reload during development

```bash
cargo install cargo-watch
cargo watch -x run
```

## Testing

Run tests with the test database:
```bash
./test.sh
```

Or manually:
```bash
# Start test database
docker compose up -d postgres-test

# Run tests
DATABASE_URL=postgresql://blackledger_test:test@localhost:5434/blackledger_test cargo test
```

## API Endpoints

### Health Check
- `GET /` - Health check and version

### Currencies
- `GET /currencies` - List all currencies
- `POST /currencies` - Create/update currency

### Ledgers
- `GET /ledgers` - List ledgers with pagination
- `POST /ledgers` - Create new ledger
- `PATCH /ledgers/{id}` - Update ledger

### Accounts
- `GET /accounts` - List accounts with filtering
- `POST /accounts` - Create new account
- `PATCH /accounts/{id}` - Update account
- `GET /accounts/balances` - Get account balances

### Transactions
- `GET /transactions` - List transactions with entries included
- `POST /transactions` - Post new transaction (returns transaction with entries)

## Project Structure

```
backend/
├── src/
│   ├── api/          # HTTP handlers and routing
│   ├── db/           # Database layer
│   ├── models/       # Domain models
│   ├── services/     # Business logic
│   ├── config.rs     # Configuration
│   ├── error.rs      # Error types
│   └── main.rs       # Application entry point
├── migrations/       # SQL migrations
└── tests/           # Integration tests
```

## Configuration

Environment variables:

- `DATABASE_URL` - PostgreSQL connection string (required)
- `PORT` - Server port (default: 8000)
- `AUTH_ENABLED` - Enable JWT authentication (default: false)
- `JWKS_URL` - JWKS endpoint URL (required if AUTH_ENABLED=true)
- `RUST_LOG` - Logging configuration

## 📖 Documentation

- **[API Documentation](./API.md)** - Complete REST API reference
- **[Roadmap](../docs/ROADMAP.md)** - Development status and future plans
- **[Rust Port Guide](../docs/rust-port.md)** - Implementation details
- **[Examples](./examples/)** - Integration examples and tutorials
- **[CLAUDE.md](../CLAUDE.md)** - AI assistant guidance

## 🏃 Examples

### Basic Accounting Example
```bash
cargo run --example basic_accounting
```

### REST API Client Example
```bash
cargo run --example rest_api_client
```

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_transaction_posting

# Run tests with code coverage
cargo tarpaulin --config tarpaulin.toml

# Quick coverage summary
cargo tarpaulin --out Stdout

# Generate HTML coverage report  
./scripts/coverage.sh

# Using cargo aliases (defined in .cargo/config.toml)
cargo coverage      # Run with full config
cargo coverage-html # Generate HTML report only
cargo cov          # Quick summary
```

### Code Coverage

The project uses `cargo-tarpaulin` for code coverage reporting. Install it with:

```bash
cargo install cargo-tarpaulin
```

Coverage configuration is defined in `tarpaulin.toml`. The coverage script (`scripts/coverage.sh`) will:
- Run database migrations
- Execute all tests with coverage tracking
- Generate an HTML report (`tarpaulin-report.html`)
- Generate LCOV and JSON formats for CI integration

Current coverage: ~42% (focusing on critical paths)

## 🚦 Production Status

The Rust port is **~90% complete** and production-ready for core accounting operations. All major features are implemented including:
- Complete CRUD operations for all entities
- Full transaction posting with validation
- Comprehensive search and pagination
- JWT authentication with JWKS support
- 51+ tests passing

Recent updates:
- Transaction `posted` field renamed to `created` for clarity
- Transaction responses now include `entries` for consistency
- Improved error handling and validation

See [ROADMAP.md](../docs/ROADMAP.md) for remaining work.

## 📄 License

MIT License - see LICENSE file for details.