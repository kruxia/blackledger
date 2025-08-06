# Blackledger REST API Documentation

## Base URL

```
http://localhost:3000/api
```

## Authentication

Most write operations require JWT authentication. Include the token in the Authorization header:

```http
Authorization: Bearer <your-jwt-token>
```

## Response Format

All responses follow a consistent format:

### Success Response
```json
{
  "data": { ... }  // For single items
  // OR
  "data": [ ... ]  // For lists
}
```

### Paginated Response
```json
{
  "data": [ ... ],
  "pagination": {
    "page": 1,
    "page_size": 20,
    "total": 100,
    "has_more": true
  }
}
```

### Error Response
```json
{
  "error": "Error message",
  "status": 400
}
```

## Endpoints

### Health Check

#### GET /api
Check API health and database connectivity.

**Response:**
```json
{
  "status": "healthy",
  "version": "0.1.0",
  "auth_enabled": false
}
```

---

### Currencies

#### POST /api/currencies
Create or update a currency.

**Authentication:** Required

**Request:**
```json
{
  "code": "USD"
}
```

**Response:** `201 Created`
```json
{
  "code": "USD"
}
```

#### GET /api/currencies
List all currencies.

**Response:**
```json
[
  { "code": "USD" },
  { "code": "EUR" },
  { "code": "GBP" }
]
```

---

### Ledgers

#### POST /api/ledgers
Create a new ledger.

**Authentication:** Required

**Request:**
```json
{
  "name": "Main Company Books"
}
```

**Response:** `201 Created`
```json
{
  "id": 1,
  "name": "Main Company Books",
  "created": "2025-08-06T10:30:00Z"
}
```

#### GET /api/ledgers
List ledgers with pagination.

**Query Parameters:**
- `page` (default: 1)
- `page_size` (default: 20)

**Response:**
```json
{
  "data": [
    {
      "id": 1,
      "name": "Main Company Books",
      "created": "2025-08-06T10:30:00Z"
    }
  ],
  "pagination": {
    "page": 1,
    "page_size": 20,
    "total": 1,
    "has_more": false
  }
}
```

#### GET /api/ledgers/{id}
Get a specific ledger.

**Response:**
```json
{
  "id": 1,
  "name": "Main Company Books",
  "created": "2025-08-06T10:30:00Z"
}
```

#### PATCH /api/ledgers/{id}
Update a ledger.

**Authentication:** Required

**Request:**
```json
{
  "name": "Updated Ledger Name"
}
```

---

### Accounts

#### POST /api/accounts
Create a new account.

**Authentication:** Required

**Request:**
```json
{
  "ledger_id": 1,
  "parent_id": null,
  "name": "Cash",
  "number": 1000,
  "normal": "DR"
}
```

**Response:** `201 Created`
```json
{
  "id": 1,
  "ledger_id": 1,
  "parent_id": null,
  "name": "Cash",
  "number": 1000,
  "created": "2025-08-06T10:30:00Z",
  "normal": "DR",
  "version": null
}
```

#### GET /api/accounts
Search and list accounts with pagination.

**Query Parameters:**
- `ledger_id` - Filter by ledger
- `parent_id` - Filter by parent account
- `name` - Search by name (partial match)
- `number` - Search by account number
- `page` (default: 1)
- `page_size` (default: 20)
- `sort_by` - Sort field (created, number, name)
- `sort_order` - asc or desc

**Response:**
```json
{
  "data": [
    {
      "id": 1,
      "ledger_id": 1,
      "parent_id": null,
      "name": "Cash",
      "number": 1000,
      "created": "2025-08-06T10:30:00Z",
      "normal": "DR",
      "version": 5
    }
  ],
  "pagination": {
    "page": 1,
    "page_size": 20,
    "total": 15,
    "has_more": false
  }
}
```

#### GET /api/accounts/{id}
Get a specific account.

#### PATCH /api/accounts/{id}
Update an account name.

**Authentication:** Required

#### GET /api/accounts/balances
Get account balances.

**Query Parameters:**
- `ledger_id` (required) - Ledger to query
- `account_ids` - Comma-separated list of account IDs

**Response:**
```json
[
  {
    "account_id": 1,
    "currency_code": "USD",
    "balance": "1500.00"
  },
  {
    "account_id": 1,
    "currency_code": "EUR",
    "balance": "850.00"
  }
]
```

---

### Transactions

#### POST /api/transactions
Post a new transaction.

**Authentication:** Required

**Request:**
```json
{
  "ledger_id": 1,
  "effective": "2025-08-06T10:30:00Z",
  "memo": "Cash sale",
  "meta": {
    "invoice": "INV-001",
    "customer": "ABC Corp"
  },
  "entries": [
    {
      "account_id": 1,
      "currency_code": "USD",
      "debit": "100.00",
      "credit": null,
      "account_version": 5
    },
    {
      "account_id": 2,
      "currency_code": "USD",
      "debit": null,
      "credit": "100.00",
      "account_version": null
    }
  ]
}
```

**Validation Rules:**
- Transaction must balance (sum of debits = sum of credits) per currency
- All accounts must exist and belong to the specified ledger
- All currencies must exist
- Each entry must have either debit OR credit (not both)
- Amounts must be positive
- Account versions (if provided) must match current version

**Response:** `201 Created`
```json
{
  "transaction": {
    "id": 1,
    "ledger_id": 1,
    "posted": "2025-08-06T10:31:00Z",
    "effective": "2025-08-06T10:30:00Z",
    "memo": "Cash sale",
    "meta": {
      "invoice": "INV-001",
      "customer": "ABC Corp",
      "audit": {
        "posted_by": "user123",
        "posted_at": "2025-08-06T10:31:00Z"
      }
    }
  },
  "entries": [
    {
      "id": 1,
      "ledger_id": 1,
      "transaction_id": 1,
      "account_id": 1,
      "currency_code": "USD",
      "debit": "100.00",
      "credit": null
    },
    {
      "id": 2,
      "ledger_id": 1,
      "transaction_id": 1,
      "account_id": 2,
      "currency_code": "USD",
      "debit": null,
      "credit": "100.00"
    }
  ]
}
```

#### GET /api/transactions
Search and list transactions with pagination.

**Query Parameters:**
- `ledger_id` - Filter by ledger
- `account_id` - Filter by account (in any entry)
- `currency_code` - Filter by currency
- `from_amount` - Minimum amount
- `to_amount` - Maximum amount
- `from_date` - Start date (ISO 8601)
- `to_date` - End date (ISO 8601)
- `page` (default: 1)
- `page_size` (default: 20)

#### GET /api/transactions/{id}
Get a specific transaction with its entries.

**Response:**
```json
{
  "transaction": { ... },
  "entries": [ ... ]
}
```

---

### Entries

#### GET /api/entries
Search and list entries with pagination.

**Query Parameters:**
- `ledger_id` - Filter by ledger
- `account_id` - Filter by account
- `transaction_id` - Filter by transaction
- `currency_code` - Filter by currency
- `from_amount` - Minimum amount (debit or credit)
- `to_amount` - Maximum amount (debit or credit)
- `page` (default: 1)
- `page_size` (default: 20)

**Response:**
```json
{
  "data": [
    {
      "id": 1,
      "ledger_id": 1,
      "transaction_id": 1,
      "account_id": 1,
      "currency_code": "USD",
      "debit": "100.00",
      "credit": null
    }
  ],
  "pagination": {
    "page": 1,
    "page_size": 20,
    "total": 150,
    "has_more": true
  }
}
```

---

## Error Codes

| Status Code | Description |
|------------|-------------|
| 200 | Success |
| 201 | Created |
| 400 | Bad Request - Validation error |
| 401 | Unauthorized - Invalid or missing token |
| 404 | Not Found - Resource doesn't exist |
| 409 | Conflict - Optimistic lock error or constraint violation |
| 500 | Internal Server Error |

## Common Error Scenarios

### Unbalanced Transaction
```json
{
  "error": "Transaction does not balance for currency USD: 50.00",
  "status": 400
}
```

### Optimistic Lock Error
```json
{
  "error": "Account version mismatch",
  "status": 409
}
```

### Invalid Currency
```json
{
  "error": "Currency XXX does not exist",
  "status": 400
}
```

## Rate Limiting

Currently not implemented. Will be added in production deployment.

## Webhooks

Not yet implemented. Planned for future release.

## SDK Support

Client libraries are planned for:
- JavaScript/TypeScript
- Python
- Go
- Ruby

## Example Workflows

### 1. Basic Sale Transaction

```bash
# Create ledger
curl -X POST http://localhost:3000/api/ledgers \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name": "My Business"}'

# Create accounts
curl -X POST http://localhost:3000/api/accounts \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "ledger_id": 1,
    "name": "Cash",
    "number": 1000,
    "normal": "DR"
  }'

# Post transaction
curl -X POST http://localhost:3000/api/transactions \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "ledger_id": 1,
    "effective": "2025-08-06T10:00:00Z",
    "memo": "Cash sale",
    "entries": [
      {
        "account_id": 1,
        "currency_code": "USD",
        "debit": "100.00",
        "credit": null
      },
      {
        "account_id": 2,
        "currency_code": "USD",
        "debit": null,
        "credit": "100.00"
      }
    ]
  }'
```

### 2. Query Account Balances

```bash
curl "http://localhost:3000/api/accounts/balances?ledger_id=1"
```

### 3. Search Transactions

```bash
curl "http://localhost:3000/api/transactions?ledger_id=1&from_date=2025-08-01&page=1&page_size=50"
```

---

*For more examples, see the [examples directory](./examples/).*