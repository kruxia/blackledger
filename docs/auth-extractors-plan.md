# Auth Extractors Implementation Plan for Complete Audit Trails

## Overview

This document outlines the implementation plan for integrating authentication extractors into all API handlers in the Blackledger Rust backend. The goal is to ensure every API request captures authenticated user context for complete audit trails, security, and compliance.

## Current State Analysis

### Existing Auth Infrastructure
- **JwtValidator**: Core JWT validation with JWKS support
- **AuthUser**: Basic extractor that requires authentication
- **OptionalAuthUser**: Currently allows anonymous access (MUST BE REMOVED)
- **auth_middleware**: Global middleware for token validation
- **Security Issue**: `handle_create_transactions` incorrectly uses `OptionalAuthUser`

### Critical Security Gaps
1. **Transaction posting allows anonymous access** - This is a critical security vulnerability
2. Most handlers lack user context extraction
3. No standardized auth requirements per endpoint
4. Missing audit context (IP, session ID, request ID)
5. No role-based access control (RBAC) integration
6. Incomplete user activity tracking

## Goals

1. **Mandatory Authentication**: ALL endpoints must require authentication - no exceptions
2. **Complete Coverage**: Every API handler must extract user context
3. **Role-Based Access**: Support different authorization levels (user, admin, service)
4. **Rich Context**: Capture user, session, IP, and request metadata
5. **Audit Integration**: Automatic audit trail generation for all operations
6. **Performance**: Minimal overhead with efficient caching
7. **Security**: Prevent all unauthorized access and track every attempt

## Architecture Design

### 1. Enhanced Auth Extractors

#### Core Extractors

```rust
// src/auth/extractors.rs

use axum::{
    async_trait,
    extract::{ConnectInfo, FromRequestParts, OriginalUri},
    http::{header::AUTHORIZATION, request::Parts, HeaderMap},
    response::{IntoResponse, Response},
};
use std::net::SocketAddr;
use uuid::Uuid;

/// Required authenticated user - returns 401 if not authenticated
/// This is the PRIMARY extractor for all endpoints
#[derive(Debug, Clone)]
pub struct RequiredAuth {
    pub user: AuthUser,
    pub context: RequestContext,
}

/// Service-to-service authentication for internal APIs
/// Used ONLY for health checks and metrics endpoints
#[derive(Debug, Clone)]
pub struct ServiceAuth {
    pub service: ServiceIdentity,
    pub context: RequestContext,
}

/// Admin-only authentication with role verification
/// For system administration tasks only
#[derive(Debug, Clone)]
pub struct AdminAuth {
    pub user: AuthUser,
    pub roles: Vec<String>,
    pub context: RequestContext,
}

// NOTE: NO OptionalAuth - all endpoints require authentication

/// Enhanced user information with roles and permissions
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: String,           // Unique user identifier (sub claim)
    pub email: Option<String>,
    pub name: Option<String>,
    pub roles: Vec<String>,    // User roles from token
    pub permissions: Vec<String>, // Computed permissions
    pub tenant_id: Option<String>, // Multi-tenancy support
    pub session_id: String,    // Session tracking
    pub token_id: String,      // Token identifier for revocation
}

/// Request context for audit trails
#[derive(Debug, Clone)]
pub struct RequestContext {
    pub request_id: Uuid,
    pub trace_id: String,
    pub client_ip: String,
    pub user_agent: Option<String>,
    pub origin: Option<String>,
    pub path: String,
    pub method: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Service identity for machine-to-machine auth
#[derive(Debug, Clone)]
pub struct ServiceIdentity {
    pub service_id: String,
    pub service_name: String,
    pub allowed_scopes: Vec<String>,
}
```

#### Extractor Implementations

```rust
// src/auth/extractors/required.rs

#[async_trait]
impl<S> FromRequestParts<S> for RequiredAuth
where
    S: Send + Sync,
    JwtValidator: FromRef<S>,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // Extract JWT validator from state
        let validator = JwtValidator::from_ref(state);
        
        // Extract authorization header
        let auth_header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .ok_or(AuthError::MissingToken)?;
        
        // Parse Bearer token
        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or(AuthError::InvalidTokenFormat)?;
        
        // Validate token and extract claims
        let claims = validator.validate_token(token).await?;
        
        // Build user from claims
        let user = AuthUser {
            id: claims.sub,
            email: claims.email,
            name: claims.name,
            roles: extract_roles(&claims),
            permissions: compute_permissions(&claims),
            tenant_id: claims.tenant_id,
            session_id: extract_session_id(&claims),
            token_id: extract_token_id(&claims),
        };
        
        // Extract request context
        let context = extract_request_context(parts)?;
        
        Ok(RequiredAuth { user, context })
    }
}
```

### 2. Enhanced JWT Claims Structure

```rust
// src/auth/claims.rs

#[derive(Debug, Serialize, Deserialize)]
pub struct EnhancedClaims {
    // Standard JWT claims
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
    pub iss: Option<String>,
    pub aud: Option<Vec<String>>,
    
    // User profile
    pub email: Option<String>,
    pub name: Option<String>,
    pub preferred_username: Option<String>,
    
    // Authorization
    pub roles: Vec<String>,
    pub scope: Option<String>,
    pub permissions: Option<Vec<String>>,
    
    // Multi-tenancy
    pub tenant_id: Option<String>,
    pub organization_id: Option<String>,
    
    // Session tracking
    pub sid: Option<String>,      // Session ID
    pub jti: Option<String>,      // JWT ID for revocation
    pub auth_time: Option<usize>, // Authentication timestamp
    
    // Custom claims
    pub custom: Option<HashMap<String, serde_json::Value>>,
}
```

### 3. Audit Trail Integration

```rust
// src/auth/audit.rs

use crate::observability::audit::{AuditLog, AuditAction, AuditResult};

pub struct AuditContext {
    pub user_id: String,
    pub user_email: Option<String>,
    pub client_ip: String,
    pub trace_id: String,
    pub session_id: String,
    pub request_path: String,
    pub request_method: String,
}

impl From<&RequiredAuth> for AuditContext {
    fn from(auth: &RequiredAuth) -> Self {
        AuditContext {
            user_id: auth.user.id.clone(),
            user_email: auth.user.email.clone(),
            client_ip: auth.context.client_ip.clone(),
            trace_id: auth.context.trace_id.clone(),
            session_id: auth.user.session_id.clone(),
            request_path: auth.context.path.clone(),
            request_method: auth.context.method.clone(),
        }
    }
}

/// Trait for automatic audit logging
#[async_trait]
pub trait Auditable {
    async fn audit_success(&self, action: AuditAction, resource: &str, resource_id: Option<&str>);
    async fn audit_failure(&self, action: AuditAction, resource: &str, error: &str);
}

#[async_trait]
impl Auditable for RequiredAuth {
    async fn audit_success(&self, action: AuditAction, resource: &str, resource_id: Option<&str>) {
        let log = AuditLog {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            trace_id: self.context.trace_id.clone(),
            user_id: Some(self.user.id.clone()),
            client_ip: self.context.client_ip.clone(),
            method: self.context.method.clone(),
            path: self.context.path.clone(),
            action,
            resource_type: resource.to_string(),
            resource_id: resource_id.map(String::from),
            result: AuditResult::Success,
            error_message: None,
            changes: None,
        };
        
        // Fire and forget audit logging
        tokio::spawn(async move {
            let _ = log.save().await;
        });
    }
    
    async fn audit_failure(&self, action: AuditAction, resource: &str, error: &str) {
        // Similar implementation for failures
    }
}
```

### 4. Handler Integration Pattern

#### Before (Current State)
```rust
pub async fn handle_create_accounts(
    State(state): State<AppState>,
    Json(input): Json<Vec<CreateAccount>>,
) -> ApiResult<(StatusCode, Json<Vec<Account>>)> {
    let accounts = create_accounts_batch(&state.pool, &input).await?;
    Ok((StatusCode::CREATED, Json(accounts)))
}
```

#### After (With Auth Extractors)
```rust
pub async fn handle_create_accounts(
    State(state): State<AppState>,
    auth: RequiredAuth,  // Auth extractor
    Json(input): Json<Vec<CreateAccount>>,
) -> ApiResult<(StatusCode, Json<Vec<Account>>)> {
    // Automatic audit trail via extractor
    auth.audit_success(
        AuditAction::Create, 
        "account", 
        None
    ).await;
    
    // Pass user context to service layer
    let accounts = create_accounts_batch(
        &state.pool, 
        &input,
        &auth.user  // User context for created_by field
    ).await?;
    
    // Log successful creation with resource IDs
    for account in &accounts {
        auth.audit_success(
            AuditAction::Create,
            "account",
            Some(&account.id.to_string())
        ).await;
    }
    
    Ok((StatusCode::CREATED, Json(accounts)))
}
```

## IMMEDIATE SECURITY FIX REQUIRED

### Critical Vulnerability
The `handle_create_transactions` endpoint currently uses `OptionalAuthUser`, allowing anonymous transaction posting. This is a **CRITICAL SECURITY VULNERABILITY** that must be fixed immediately.

### Emergency Fix Steps
1. Replace `OptionalAuthUser` with `RequiredAuth` in `handle_create_transactions`
2. Remove `OptionalAuthUser` struct from `src/auth/mod.rs`
3. Update all tests to include authentication headers
4. Deploy fix immediately to all environments

### Code Changes Required

```rust
// src/api/handlers/transactions.rs
// BEFORE (VULNERABLE):
pub async fn handle_create_transactions(
    State(state): State<AppState>,
    OptionalAuthUser(auth_user): OptionalAuthUser,  // SECURITY ISSUE
    Json(input): Json<Vec<CreateTransaction>>,
) -> ApiResult<(StatusCode, Json<Vec<Transaction>>)> {
    let user_id = auth_user.as_ref().map(|u| u.sub.as_str());  // Allows None!
    // ...
}

// AFTER (SECURE):
pub async fn handle_create_transactions(
    State(state): State<AppState>,
    auth: RequiredAuth,  // MANDATORY authentication
    Json(input): Json<Vec<CreateTransaction>>,
) -> ApiResult<(StatusCode, Json<Vec<Transaction>>)> {
    let user_id = &auth.user.id;  // Always has authenticated user
    
    // Add audit logging
    auth.audit_success(
        AuditAction::Post,
        "transaction",
        None
    ).await;
    
    // ...
}
```

## Implementation Plan

### Phase 1: Core Infrastructure (Day 1)

1. **IMMEDIATE SECURITY FIX**
   - Remove OptionalAuthUser from codebase
   - Update handle_create_transactions to use RequiredAuth
   - Ensure NO endpoints allow anonymous access

2. **Create auth extractors module structure**
   ```
   src/auth/
   ├── extractors/
   │   ├── mod.rs
   │   ├── required.rs    # Primary extractor for all endpoints
   │   ├── admin.rs       # Admin-only operations
   │   └── service.rs     # Service accounts only
   ├── claims.rs
   ├── context.rs
   └── audit.rs
   ```

3. **Implement base extractors**
   - RequiredAuth with full user context (PRIMARY)
   - AdminAuth with role verification
   - ServiceAuth for health/metrics only

3. **Add request context extraction**
   - Client IP from connection info
   - Request ID generation
   - Trace ID propagation
   - User agent parsing

### Phase 2: JWT Enhancement (Day 2)

1. **Extend Claims structure**
   - Add roles and permissions
   - Include session tracking fields
   - Support multi-tenancy claims
   - Add custom claims map

2. **Implement claim validation**
   - Role-based access checks
   - Permission computation
   - Token revocation checks
   - Session validation

3. **Add caching layer**
   - Cache validated tokens (5-minute TTL)
   - Cache user permissions
   - Cache JWKS keys

### Phase 3: Audit Integration (Day 3)

1. **Create audit traits**
   - Auditable trait for automatic logging
   - AuditContext conversion
   - Async audit recording

2. **Implement audit macros**
   ```rust
   #[audit(action = "create", resource = "account")]
   pub async fn handle_create_accounts(...) { }
   ```

3. **Add audit middleware**
   - Capture all requests
   - Log authentication attempts
   - Track API usage patterns

### Phase 4: Handler Migration (Days 4-5)

#### Priority 1: CRITICAL SECURITY FIXES (IMMEDIATE)
1. **Fix transaction handlers**
   - `handle_create_transactions` → RequiredAuth (CRITICAL - currently using OptionalAuth)
   - Remove all OptionalAuthUser usage
   - Add full audit logging

#### Priority 2: Write Operations (Day 4)
1. **Account handlers**
   - `handle_create_accounts` → RequiredAuth
   - `handle_update_account` → RequiredAuth

2. **Ledger handlers**
   - `handle_create_ledgers` → RequiredAuth
   - `handle_update_ledger` → RequiredAuth

#### Priority 3: Read Operations (Day 5)
1. **Search handlers**
   - `handle_search_transactions` → RequiredAuth
   - `handle_search_accounts` → RequiredAuth
   - `handle_search_ledgers` → RequiredAuth
   - Track all data access for compliance

2. **Get handlers**
   - `handle_get_balances` → RequiredAuth
   - `handle_get_transaction` → RequiredAuth
   - Enforce data access controls

#### Priority 4: Admin Operations (Day 5)
1. **Currency management**
   - `handle_create_currencies` → AdminAuth
   - Restricted to admin roles only

2. **System operations**
   - Database migrations → AdminAuth
   - Health checks → ServiceAuth (only endpoint without user auth)
   - Metrics endpoint → ServiceAuth

### Phase 5: Testing & Validation (Day 6)

1. **Unit tests for extractors**
   ```rust
   #[tokio::test]
   async fn test_required_auth_valid_token() {
       // Test successful extraction
   }
   
   #[tokio::test]
   async fn test_required_auth_missing_token() {
       // Test 401 response
   }
   
   #[tokio::test]
   async fn test_admin_auth_insufficient_roles() {
       // Test 403 response
   }
   ```

2. **Integration tests**
   - End-to-end auth flow
   - Audit trail verification
   - Performance benchmarks

3. **Security testing**
   - Token manipulation attempts
   - Expired token handling
   - Role bypass attempts

### Phase 6: Monitoring & Observability (Day 7)

1. **Add auth metrics**
   ```rust
   pub static AUTH_ATTEMPTS: Lazy<CounterVec> = Lazy::new(|| {
       register_counter_vec!(
           "auth_attempts_total",
           "Authentication attempts",
           &["result", "reason"]
       ).unwrap()
   });
   
   pub static AUTH_LATENCY: Lazy<HistogramVec> = Lazy::new(|| {
       register_histogram_vec!(
           "auth_validation_duration_seconds",
           "Time to validate authentication",
           &["auth_type"]
       ).unwrap()
   });
   ```

2. **Create dashboards**
   - Authentication success/failure rates
   - User activity patterns
   - Suspicious access patterns
   - Token validation performance

3. **Set up alerts**
   - Failed authentication spikes
   - Unauthorized access attempts
   - Unusual activity patterns

## Migration Guide

### Step-by-Step Handler Migration

1. **Identify auth requirements**
   ```rust
   // ALL endpoints require authentication:
   // - RequiredAuth: Standard for all user-facing endpoints
   // - AdminAuth: System administration only
   // - ServiceAuth: Health/metrics endpoints only
   // NO PUBLIC ENDPOINTS - NO OPTIONAL AUTH
   ```

2. **Add extractor to handler signature**
   ```rust
   // Before
   pub async fn handle(State(state): State<AppState>, ...)
   
   // After
   pub async fn handle(
       State(state): State<AppState>,
       auth: RequiredAuth,  // Add auth extractor
       ...
   )
   ```

3. **Pass user context to service layer**
   ```rust
   // Update service functions to accept user context
   create_account(&pool, &input, &auth.user).await?
   ```

4. **Add audit logging**
   ```rust
   // Log the action
   auth.audit_success(AuditAction::Create, "account", Some(&id)).await;
   ```

5. **Update tests**
   ```rust
   // Add auth headers to test requests
   let response = client
       .post("/accounts")
       .header("Authorization", "Bearer test-token")
       .json(&input)
       .send()
       .await;
   ```

## Configuration

### Environment Variables

```bash
# Authentication
AUTH_ENABLED=true
AUTH_JWKS_URL=https://auth.example.com/.well-known/jwks.json
AUTH_ISSUER=https://auth.example.com
AUTH_AUDIENCE=blackledger-api

# Token Validation
AUTH_TOKEN_CACHE_TTL=300  # 5 minutes
AUTH_VALIDATE_EXPIRY=true
AUTH_VALIDATE_AUDIENCE=true
AUTH_VALIDATE_ISSUER=true
AUTH_CLOCK_SKEW_SECONDS=60

# Session Management
AUTH_SESSION_TIMEOUT=3600  # 1 hour
AUTH_REQUIRE_SESSION=false
AUTH_SINGLE_SESSION=false  # Enforce single session per user

# Audit
AUDIT_ENABLED=true
AUDIT_ASYNC=true  # Non-blocking audit writes
AUDIT_INCLUDE_REQUEST_BODY=false  # PII consideration
AUDIT_INCLUDE_RESPONSE_BODY=false

# Rate Limiting (per user)
AUTH_RATE_LIMIT_ENABLED=true
AUTH_RATE_LIMIT_PER_MINUTE=600
AUTH_RATE_LIMIT_BURST=100
```

### Database Schema

```sql
-- Enhanced audit log table
CREATE TABLE audit_logs (
    id UUID PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL,
    trace_id VARCHAR(64) NOT NULL,
    user_id VARCHAR(255),
    user_email VARCHAR(255),
    session_id VARCHAR(64),
    client_ip INET NOT NULL,
    user_agent TEXT,
    method VARCHAR(10) NOT NULL,
    path TEXT NOT NULL,
    action VARCHAR(50) NOT NULL,
    resource_type VARCHAR(50) NOT NULL,
    resource_id VARCHAR(255),
    result VARCHAR(20) NOT NULL,
    error_message TEXT,
    request_body JSONB,  -- Optional, be careful with PII
    response_status INT,
    duration_ms INT,
    
    -- Indexes for querying
    INDEX idx_audit_user_id (user_id),
    INDEX idx_audit_timestamp (timestamp),
    INDEX idx_audit_trace_id (trace_id),
    INDEX idx_audit_session (session_id),
    INDEX idx_audit_resource (resource_type, resource_id)
);

-- User sessions table for tracking
CREATE TABLE user_sessions (
    session_id VARCHAR(64) PRIMARY KEY,
    user_id VARCHAR(255) NOT NULL,
    token_id VARCHAR(64) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    last_activity TIMESTAMPTZ NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    ip_address INET,
    user_agent TEXT,
    revoked BOOLEAN DEFAULT FALSE,
    revoked_at TIMESTAMPTZ,
    revoked_reason TEXT,
    
    INDEX idx_session_user (user_id),
    INDEX idx_session_token (token_id),
    INDEX idx_session_expires (expires_at)
);
```

## Security Considerations

### 1. Token Security
- Validate token signature with JWKS
- Check token expiration with clock skew
- Verify issuer and audience claims
- Implement token revocation checks
- Rate limit validation attempts

### 2. Session Management
- Track active sessions per user
- Implement session timeout
- Support forced logout/session invalidation
- Detect concurrent session usage
- Alert on suspicious session patterns

### 3. Audit Security
- Ensure audit logs are immutable
- Encrypt sensitive data in audit logs
- Implement audit log retention policies
- Secure audit query endpoints
- Monitor for audit tampering

### 4. Rate Limiting
- Per-user rate limits
- Per-IP rate limits for anonymous
- Endpoint-specific limits
- Progressive backoff for failures
- Bypass for service accounts

### 5. Data Privacy
- Mask sensitive data in logs
- Implement PII detection
- Support GDPR right to erasure
- Encrypt authentication tokens at rest
- Secure token transmission (TLS only)

## Performance Optimization

### 1. Caching Strategy
```rust
use moka::future::Cache;

pub struct AuthCache {
    // Cache validated tokens (5 min TTL)
    tokens: Cache<String, ValidatedToken>,
    
    // Cache user permissions (10 min TTL)
    permissions: Cache<String, Vec<String>>,
    
    // Cache JWKS keys (1 hour TTL)
    jwks: Cache<String, DecodingKey>,
}
```

### 2. Async Processing
- Non-blocking audit logging
- Background session updates
- Async permission computation
- Parallel token validation

### 3. Database Optimization
- Batch audit log inserts
- Connection pooling for auth queries
- Indexed lookups for sessions
- Partitioned audit tables by date

## Monitoring & Alerts

### Key Metrics
1. **Authentication Metrics**
   - Success/failure rate
   - Validation latency
   - Cache hit rates
   - Token expiration patterns

2. **Authorization Metrics**
   - Permission check latency
   - Role verification failures
   - Access denied patterns

3. **Audit Metrics**
   - Audit write latency
   - Failed audit writes
   - Audit storage growth

### Alert Conditions
1. **Security Alerts**
   - Multiple failed auth attempts
   - Expired token usage spike
   - Invalid signature attempts
   - Unusual access patterns

2. **Performance Alerts**
   - Auth latency > 100ms
   - Cache miss rate > 20%
   - Audit queue backup

3. **Operational Alerts**
   - JWKS fetch failures
   - Database connection issues
   - Audit storage full

## Testing Strategy

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_required_auth_extraction() {
        // Test successful token extraction
    }
    
    #[tokio::test]
    async fn test_permission_computation() {
        // Test permission derivation from roles
    }
    
    #[tokio::test]
    async fn test_audit_context_creation() {
        // Test audit context generation
    }
}
```

### Integration Tests
```rust
#[tokio::test]
async fn test_end_to_end_auth_flow() {
    // 1. Generate JWT token
    // 2. Make authenticated request
    // 3. Verify audit log created
    // 4. Check response includes user context
}
```

### Load Tests
```rust
#[tokio::test]
async fn test_auth_performance_under_load() {
    // Simulate 1000 concurrent requests
    // Measure auth validation latency
    // Verify no auth failures under load
}
```

## Rollout Plan

### Week 1: Foundation
- Days 1-2: Implement core extractors
- Days 3-4: Enhance JWT handling
- Days 5-6: Integrate audit system
- Day 7: Testing and documentation

### Week 2: Migration
- Migrate critical write operations
- Update all read operations
- Add admin endpoints
- Performance testing

### Gradual Rollout
1. **Stage 1**: Deploy to development
2. **Stage 2**: Limited staging rollout
3. **Stage 3**: Production with feature flag
4. **Stage 4**: Enable for 10% of users
5. **Stage 5**: Full production rollout

## Success Metrics

1. **Coverage**: 100% of endpoints have auth extractors
2. **Audit Completeness**: Every state change is logged
3. **Performance**: <10ms auth overhead per request
4. **Security**: Zero unauthorized access after implementation
5. **Compliance**: Full audit trail for all operations
6. **Developer Experience**: Simple, consistent auth pattern

## Maintenance & Evolution

### Regular Tasks
1. **Weekly**: Review authentication failures
2. **Monthly**: Audit log analysis and reporting
3. **Quarterly**: Security audit and penetration testing
4. **Annually**: Compliance certification renewal

### Future Enhancements
1. **Multi-factor Authentication (MFA)**
2. **Biometric authentication support**
3. **OAuth2/OIDC provider integration**
4. **Fine-grained permissions (ABAC)**
5. **Zero-trust architecture**
6. **Blockchain-based audit trails**