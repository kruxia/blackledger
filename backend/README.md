# Blackledger Rust Backend

Rust implementation of the Blackledger double-entry accounting API using Axum and SQLx.

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
- `GET /api` - Health check and version

### Currencies
- `GET /api/currencies` - List all currencies
- `POST /api/currencies` - Create/update currency

### Ledgers
- `GET /api/ledgers` - List ledgers with pagination
- `POST /api/ledgers` - Create new ledger
- `GET /api/ledgers/{id}` - Get ledger by ID
- `PATCH /api/ledgers/{id}` - Update ledger

### Accounts
- `GET /api/accounts` - List accounts with filtering
- `POST /api/accounts` - Create new account
- `GET /api/accounts/{id}` - Get account by ID
- `PATCH /api/accounts/{id}` - Update account
- `GET /api/accounts/balances` - Get account balances

### Transactions
- `GET /api/transactions` - List transactions
- `POST /api/transactions` - Post new transaction
- `GET /api/transactions/{id}` - Get transaction with entries

### Entries
- `GET /api/entries` - List entries with filtering

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