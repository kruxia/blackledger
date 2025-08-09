# Test Coverage Improvement Plan

## Current State (Updated)
- **Overall Coverage**: 74.89% (656/876 lines)
- **Target Coverage**: 90%+ (788+ lines)
- **Gap**: 132 lines need coverage
- **Progress**: Improved from 58.37% to 74.89% (+16.52%)
- **Latest Achievement**: Entry queries module improved to 78.3% coverage (was 10.5%)
- **Note**: Total line count decreased from 891 to 876 lines (likely due to removal of unused get_entries_by_transaction function)

## Priority Areas for Coverage Improvement

### Critical Path Components (Priority 1)

#### 1. Main Entry Point (`src/main.rs`) ✅ IMPROVED
- **Current**: 0/7 lines (0%)
- **Impact**: High - Application startup and configuration
- **Uncovered**: Lines 5, 7, 10-11, 14, 17, 19
- **Status**: Refactored into `app.rs` module for better testability
- **Note**: Main function delegates to tested `app` module functions

#### 2. Transaction Posting Service (`src/services/posting.rs`)
- **Current**: 45/68 lines (66.2%)
- **Impact**: Critical - Core business logic
- **Key Uncovered Areas**:
  - Error handling paths (52, 70, 82)
  - Rollback scenarios (111, 113-118, 122-123, 125-127, 131, 135-138, 142)
  - Database operations (239, 270)
- **Test Strategy**:
  - Additional error path coverage needed
  - More rollback testing scenarios
  - Database failure simulations

#### 3. Authentication Module (`src/auth/mod.rs`)
- **Current**: 44/65 lines (67.7%)
- **Impact**: Critical - Security
- **Key Uncovered Areas**:
  - JWKS fetching and key setup (52, 57-59)
  - JWT validation failures (79, 81, 84, 88, 92, 96-97)
  - Auth extraction from request (125, 132, 171-174, 176-179)
- **Test Strategy**:
  - Additional JWT validation edge cases
  - JWKS rotation scenarios
  - Request extraction error paths

### Database Query Layer (Priority 2)

#### 4. Entry Queries (`src/db/queries/entry.rs`)
- **Current**: 18/23 lines (78.3%)
- **Impact**: High - Core data access
- **Uncovered Lines**: 29-31, 40, 62
- **Test Coverage Added**:
  - ✅ get_entries_by_account with pagination
  - ✅ get_entries_for_transactions (bulk fetching)
  - ✅ Empty result handling
  - ✅ Non-existent entity handling
  - ✅ Ordering verification
  - ✅ Multi-currency support
  - ✅ Decimal precision preservation
  - ✅ Large dataset handling
- **Still Needed**: Result mapping code (lines 29-31, 40, 62) - Note: These lines are executed in tests but may not be properly detected by coverage tool

#### 5. Transaction Queries (`src/db/queries/transaction.rs`)
- **Current**: 45/81 lines (55.6%)
- **Impact**: High - Transaction management
- **Key Uncovered Areas**:
  - Basic query building (14, 16, 18-20, 22, 24-25, 31, 35, 37)
  - Search filters (41-42, 48, 50, 52-55, 61-67, 69-71, 73, 79-80, 84)
  - Complex operations (117, 163, 199)
- **Test Strategy**:
  - Search with multiple filters
  - Pagination edge cases
  - Empty result sets
  - Invalid filter combinations

#### 6. Ledger Queries (`src/db/queries/ledger.rs`)
- **Current**: 55/83 lines (66.3%)
- **Impact**: Medium - Ledger management
- **Key Uncovered Areas**:
  - Create ledger error paths (19-22)
  - Update error handling (60-66, 69-72)
  - Search filters (90-91, 108, 112, 114, 116, 118, 125, 127, 132-134, 136)
- **Test Strategy**:
  - Database error simulation
  - Complex search scenarios
  - Concurrent ledger operations

### API Handlers (Priority 3)

#### 7. Account Handlers (`src/api/handlers/accounts.rs`) ✅ COMPLETE
- **Current**: 16/16 lines (100%)
- **Impact**: Medium - User-facing API
- **Status**: Fully covered

#### 8. Currency Handlers (`src/api/handlers/currencies.rs`) ✅ COMPLETE
- **Current**: 14/14 lines (100%)
- **Impact**: Low - Simple CRUD
- **Status**: Fully covered

#### 9. Account Queries (`src/db/queries/account.rs`)
- **Current**: 129/164 lines (78.7%)
- **Impact**: High - Account management
- **Key Uncovered Areas**:
  - Error handling paths (31-36, 39, 41-42, 85, 107, 119, 141, 153)
  - Search filter edge cases (167, 180, 193, 206, 210-213, 219, 233, 235, 240-242, 244, 250-251, 258-259)
  - Balance calculations (321, 412)
- **Test Strategy**:
  - Database constraint violations
  - Complex filter combinations
  - Edge cases in balance aggregation

### Utility Components (Priority 4)

#### 10. Error Handling (`src/error.rs`)
- **Current**: 14/29 lines (48.3%)
- **Impact**: Medium - Cross-cutting concern
- **Uncovered**: Error conversions (50-52, 54, 56-57, 64-65, 69-71, 75-76, 78-79)
- **Test Strategy**:
  - Test all error type conversions
  - HTTP status code mapping
  - Error message formatting

#### 11. Validation Service (`src/services/validation.rs`)
- **Current**: 64/74 lines (86.5%)
- **Impact**: Medium - Data integrity
- **Uncovered**: Edge cases (70-72, 91-93, 177-179, 205)
- **Test Strategy**:
  - Boundary value testing
  - Invalid data combinations
  - Regex pattern edge cases

#### 12. App Module (`src/app.rs`) - NEW
- **Current**: 3/25 lines (12%)
- **Impact**: High - Application initialization
- **Uncovered**: Most initialization code (11-12, 14-15, 17, 27, 31-32, 38-39, 45, 50-53, 55, 59-61, 63-64, 66)
- **Test Strategy**:
  - Server startup with various configurations
  - Database pool creation failures
  - Migration failures
  - JWT validator initialization

## Implementation Timeline (Revised)

### Week 1: Critical Components
- [x] Main entry point refactored into testable app module
- [ ] App module comprehensive tests (20 lines to cover)
- [ ] Authentication module comprehensive tests (15 lines to cover)
- [ ] Transaction posting service edge cases (16 lines to cover)

### Week 2: Database Layer
- [ ] Entry queries remaining lines (3 lines to cover)
- [ ] Transaction queries search and pagination (28 lines to cover)
- [ ] Account queries remaining edge cases (19 lines to cover)
- [ ] Ledger queries error handling (20 lines to cover)

### Week 3: API and Integration
- [x] API handlers fully covered
- [ ] API middleware auth tests (3 lines to cover)
- [ ] API pagination tests (4 lines to cover)
- [ ] API search tests (4 lines to cover)
- [ ] End-to-end integration tests
- [ ] Concurrent operation tests

### Week 4: Polish and Documentation
- [ ] Error handling improvements (12 lines to cover)
- [ ] Validation edge cases (7 lines to cover)
- [ ] Currency queries (4 lines to cover)
- [ ] Coverage report generation
- [ ] Test documentation

## Test File Organization

```
tests/
├── unit/
│   ├── auth_test.rs
│   ├── error_handling_test.rs
│   └── validation_test.rs
├── integration/
│   ├── posting_service_test.rs
│   ├── concurrent_operations_test.rs
│   └── api_error_handling_test.rs
├── database/
│   ├── entry_queries_test.rs
│   ├── transaction_queries_test.rs
│   └── query_edge_cases_test.rs
└── e2e/
    ├── full_workflow_test.rs
    └── startup_shutdown_test.rs
```

## Success Metrics

1. **Coverage Goals**:
   - Overall: 90%+ coverage (need 191+ more lines)
   - Critical paths: 95%+ coverage
   - Error handling: 100% coverage
   - **Current Progress**: 68.57% → 90% = +21.43% needed

2. **Test Quality**:
   - All edge cases documented
   - Concurrent scenarios tested
   - Error paths validated
   - Performance benchmarks established

3. **Maintainability**:
   - Clear test naming conventions
   - Reusable test fixtures
   - Documented test scenarios
   - CI/CD integration

## Coverage Summary by Module

| Module                      | Current  |   Lines   | Target | Gap  |
|-----------------------------|----------|-----------|--------|------|
| API CORS                    | 100%     |   6/6     | ✅     | 0    |
| API Handlers                | 100%     |  55/55    | ✅     | 0    |
| API Middleware Auth         | 70%      |   7/10    | 90%    | 3    |
| API Pagination              | 60%      |   6/10    | 90%    | 4    |
| API Search                  | 92.9%    |  52/56    | 95%    | 4    |
| API Core                    | 100%     |  30/30    | ✅     | 0    |
| App Module                  | 12%      |   3/25    | 90%    | 20   |
| Auth                        | 67.7%    |  44/65    | 90%    | 15   |
| Config                      | 100%     |  10/10    | ✅     | 0    |
| DB Core                     | 100%     |   6/6     | ✅     | 0    |
| DB Query Account            | 78.7%    | 129/164   | 90%    | 19   |
| DB Query Currency           | 83.3%    |  50/60    | 90%    | 4    |
| DB Query Entry              | 78.3%    |  18/23    | 90%    | 3    |
| DB Query Ledger             | 66.3%    |  55/83    | 90%    | 20   |
| DB Query Transaction        | 55.6%    |  45/81    | 90%    | 28   |
| Error                       | 48.3%    |  14/29    | 90%    | 12   |
| Main                        | 0%       |   0/7     | N/A    | 7    |
| Models                      | 100%     |  14/14    | ✅     | 0    |
| Services Posting            | 66.2%    |  45/68    | 90%    | 16   |
| Services Validation         | 90.5%    |  67/74    | ✅     | 0    |
| **Total**                   | **74.89%**| **656/876** | **90%** | **132** |

## Special Considerations

### Database Constraints
Some lines in query modules (e.g., `account.rs` lines 39, 85, 119) handle invalid database states that are prevented by constraints. These may remain uncovered as they require bypassing SQLx safety features.

### Authentication Testing
The auth module requires careful mocking of external JWKS endpoints and token generation. Consider using a test JWKS server or stubbing responses.

### Concurrent Testing
Transaction posting and account versioning require sophisticated concurrent testing to validate optimistic locking and race condition handling.

## Tools and Infrastructure

- **Coverage Tool**: cargo-tarpaulin
- **Test Database**: PostgreSQL with test migrations
- **Mocking**: mockito for external services
- **Benchmarking**: criterion for performance tests
- **CI Integration**: GitHub Actions with coverage reporting

## Next Steps

1. **Immediate Focus** (51 lines - Critical Path):
   - App module initialization tests (20 lines)
   - Authentication module edge cases (15 lines)
   - Transaction posting error paths (16 lines)

2. **Secondary Focus** (70 lines - Database Layer):
   - Entry queries remaining mapping (3 lines)
   - Transaction queries search filters (28 lines)
   - Account query edge cases (19 lines)
   - Ledger query error handling (20 lines)

3. **Final Push** (23 lines - Quick Wins):
   - Error handling improvements (12 lines)
   - API middleware auth (3 lines)
   - API pagination (4 lines)
   - Currency queries (4 lines)
   - Achieve 90% overall coverage

4. **Infrastructure**:
   - Set up coverage gates at 85% minimum
   - Automated coverage reporting in CI
   - Test data factories for complex scenarios