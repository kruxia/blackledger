# Test Coverage Improvement Plan

## Current State (Updated)
- **Overall Coverage**: 68.57% (611/891 lines)
- **Target Coverage**: 90%+ (802+ lines)
- **Gap**: 191+ lines need coverage
- **Progress**: Improved from 58.37% to 68.57% (+10.2%)

## Priority Areas for Coverage Improvement

### Critical Path Components (Priority 1)

#### 1. Main Entry Point (`src/main.rs`) ✅ IMPROVED
- **Current**: 0/7 lines (0%)
- **Impact**: High - Application startup and configuration
- **Uncovered**: Lines 5, 7, 10-11, 14, 17, 19
- **Status**: Refactored into `app.rs` module for better testability
- **Note**: Main function delegates to tested `app` module functions

#### 2. Transaction Posting Service (`src/services/posting.rs`)
- **Current**: 39/68 lines (57.4%)
- **Impact**: Critical - Core business logic
- **Key Uncovered Areas**:
  - Error handling paths (52, 68, 70, 82, 102)
  - Transaction rollback scenarios (108-109, 111-118, 122-123, 125-127, 130-131)
  - Error conversion and mapping (135-138, 142, 239, 270)
- **Test Strategy**:
  - Concurrent posting scenarios
  - Rollback testing with failures at different stages
  - Edge cases with zero amounts
  - Multi-currency transaction scenarios

#### 3. Authentication Module (`src/auth/mod.rs`)
- **Current**: 22/65 lines (33.8%)
- **Impact**: Critical - Security
- **Key Uncovered Areas**:
  - JWKS fetching and key setup (52, 57-59)
  - JWT validation failures (63-64, 66-73, 77, 79, 81, 84, 88, 92, 96-97)
  - Auth extraction from request (114-115, 122, 125, 132, 157-162, 166-167, 171-174, 176-179)
- **Test Strategy**:
  - Mock invalid JWTs
  - Expired token tests
  - JWKS rotation scenarios
  - Missing claims tests
  - Request extraction edge cases

### Database Query Layer (Priority 2)

#### 4. Entry Queries (`src/db/queries/entry.rs`)
- **Current**: 4/38 lines (10.5%)
- **Impact**: High - Core data access
- **Uncovered**: Most query functions (7, 11, 20-21, 23-32, 34, 37, 43, 53-54, 56-57, 59-68, 70, 79, 92)
- **Test Strategy**:
  - CRUD operations for entries
  - Bulk entry creation
  - Query filtering tests
  - Error handling for constraint violations

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
  - Search filters (90-91, 108, 112, 114, 116, 118, 125, 127)
  - Complex query operations (132-134, 136)
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
- [ ] App module comprehensive tests (13 lines to cover)
- [ ] Authentication module comprehensive tests (43 lines to cover)
- [ ] Transaction posting service edge cases (29 lines to cover)

### Week 2: Database Layer
- [ ] Entry queries full coverage (34 lines to cover)
- [ ] Transaction queries search and pagination (36 lines to cover)
- [ ] Account queries remaining edge cases (35 lines to cover)
- [ ] Ledger queries error handling (28 lines to cover)

### Week 3: API and Integration
- [x] API handlers fully covered
- [ ] API middleware auth tests (3 lines to cover)
- [ ] API pagination tests (4 lines to cover)
- [ ] API search tests (4 lines to cover)
- [ ] End-to-end integration tests
- [ ] Concurrent operation tests

### Week 4: Polish and Documentation
- [ ] Error handling improvements (15 lines to cover)
- [ ] Validation edge cases (10 lines to cover)
- [ ] Currency queries (10 lines to cover)
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

| Module         | Current  |   Lines   | Target | Gap  |
|----------------|----------|-----------|--------|------|
| API Handlers   | 100%     |  55/55    | ✅     | 0    |
| API Middleware | 70%      |   7/10    | 90%    | 3    |
| API Core       | 92.9%    |  52/56    | 95%    | 4    |
| App Module     | 12%      |   3/25    | 90%    | 20   |
| Auth           | 33.8%    |  22/65    | 90%    | 37   |
| Config         | 100%     |  10/10    | ✅     | 0    |
| DB Core        | 100%     |   6/6     | ✅     | 0    |
| DB Queries     | 66.3%    | 283/426   | 90%    | 100  |
| Error          | 48.3%    |  14/29    | 90%    | 12   |
| Main           | 0%       |   0/7     | N/A    | 7    |
| Models         | 100%     |  14/14    | ✅     | 0    |
| Services       | 68.1%    | 103/142   | 90%    | 25   |
| **Total**      | **68.57%**| **611/891** | **90%** | **191** |

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

1. **Immediate Focus** (85 lines - Critical Path):
   - App module initialization tests (20 lines)
   - Authentication module with mocked JWKS (37 lines)
   - Transaction posting error paths (28 lines)

2. **Secondary Focus** (100 lines - Database Layer):
   - Entry queries CRUD operations (34 lines)
   - Transaction search filters (36 lines)
   - Account query edge cases (30 lines)

3. **Final Push** (6 lines - Quick Wins):
   - API middleware auth (3 lines)
   - API pagination (4 lines)
   - Achieve 90% overall coverage

4. **Infrastructure**:
   - Set up coverage gates at 85% minimum
   - Automated coverage reporting in CI
   - Test data factories for complex scenarios