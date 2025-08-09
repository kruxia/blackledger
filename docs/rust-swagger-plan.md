# Rust Backend Swagger/OpenAPI Implementation Plan

## Overview
This document outlines the implementation plan for adding Swagger/OpenAPI documentation to the Blackledger Rust backend using Axum. The implementation will provide interactive API documentation with try-it-out functionality and Bearer token authentication support.

## Library Selection

### Recommended: utoipa + utoipa-swagger-ui
**utoipa** is the most mature and actively maintained OpenAPI library for Rust with excellent Axum integration.

#### Advantages:
- Derive macros for automatic schema generation from Rust types
- Native Axum integration
- Built-in Swagger UI support via utoipa-swagger-ui
- Supports OpenAPI 3.0 specification
- Active development and community support
- Compile-time OpenAPI spec generation

#### Required Dependencies:
```toml
utoipa = { version = "4", features = ["axum_extras", "chrono", "decimal"] }
utoipa-swagger-ui = { version = "6", features = ["axum"] }
utoipa-redoc = { version = "3", features = ["axum"] }  # Optional: ReDoc UI
utoipa-rapidoc = { version = "3", features = ["axum"] }  # Optional: RapiDoc UI
```

### Alternative: aide
A newer library with good Axum integration but less mature ecosystem.

## Implementation Steps

### Phase 1: Core Setup

#### 1.1 Add Dependencies
- Add utoipa and utoipa-swagger-ui to Cargo.toml
- Ensure features support rust_decimal and chrono types

#### 1.2 Create OpenAPI Configuration Module
- Create `src/openapi.rs` module
- Define OpenAPI info, servers, and security schemes
- Configure Bearer token authentication scheme

```rust
// Example structure
pub fn create_openapi_spec() -> utoipa::openapi::OpenApi {
    #[derive(OpenApi)]
    #[openapi(
        info(
            title = "Blackledger API",
            version = "1.0.0",
            description = "Double-entry accounting REST API"
        ),
        servers(
            (url = "http://localhost:3000", description = "Local development"),
            (url = "https://api.blackledger.io", description = "Production")
        ),
        components(
            security_schemes(
                ("bearer_auth" = (
                    type = "http",
                    scheme = "bearer",
                    bearer_format = "JWT"
                ))
            )
        )
    )]
    struct ApiDoc;
    
    ApiDoc::openapi()
}
```

### Phase 2: Schema Generation

#### 2.1 Add Schema Derives to Models
- Add `#[derive(utoipa::ToSchema)]` to all request/response models
- Configure decimal and datetime field types
- Add descriptions and examples to fields

```rust
#[derive(Serialize, Deserialize, ToSchema)]
#[schema(example = json!({"name": "General Ledger", "description": "Main ledger"}))]
pub struct CreateLedger {
    #[schema(description = "Unique ledger name")]
    pub name: String,
    #[schema(description = "Optional ledger description")]
    pub description: Option<String>,
}
```

#### 2.2 Handle Special Types
- Configure rust_decimal::Decimal serialization
- Configure chrono::DateTime formatting
- Handle UUID types appropriately

### Phase 3: Endpoint Documentation

#### 3.1 Document Handler Functions
- Add `#[utoipa::path]` macros to all handler functions
- Define operation metadata (summary, description, tags)
- Specify request/response schemas
- Add security requirements for protected endpoints

```rust
#[utoipa::path(
    post,
    path = "/api/ledgers",
    request_body = CreateLedger,
    responses(
        (status = 201, description = "Ledger created", body = Ledger),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Ledgers"
)]
async fn create_ledger(
    // handler implementation
) -> Result<Json<Ledger>, AppError> {
    // ...
}
```

#### 3.2 Group Endpoints by Tags
- Ledgers
- Accounts
- Transactions
- Currencies
- Search Operations

### Phase 4: Swagger UI Integration

#### 4.1 Mount Swagger UI Route
- Add Swagger UI to the Axum router
- Configure at `/api-docs` path
- Ensure static assets are served correctly

```rust
use utoipa_swagger_ui::SwaggerUi;

let app = Router::new()
    .merge(SwaggerUi::new("/api-docs")
        .url("/api-docs/openapi.json", create_openapi_spec()));
```

#### 4.2 Configure Authentication UI
- Enable "Authorize" button in Swagger UI
- Configure Bearer token input field
- Ensure token is sent with try-it-out requests

### Phase 5: Authentication Integration

#### 5.1 Security Middleware Integration
- Ensure auth middleware works with Swagger UI requests
- Allow unauthenticated access to /api-docs/* paths
- Allow unauthenticated access to /api-docs/openapi.json paths

#### 5.2 Token Flow Documentation
- Document how to obtain JWT tokens
- Provide example authentication flow
- Include token refresh endpoints if applicable

### Phase 6: Advanced Features

#### 6.1 Request/Response Examples
- Add multiple examples for complex endpoints
- Show different currency scenarios
- Demonstrate error responses

#### 6.2 Search Parameter Documentation
- Document regex pattern syntax
- Provide search filter examples
- Show pagination parameters

#### 6.3 Webhook/Callback Documentation (if applicable)
- Document async operations
- Define callback schemas

### Phase 7: Testing and Validation

#### 7.1 OpenAPI Spec Validation
- Validate generated OpenAPI spec against OpenAPI 3.0 schema
- Use online validators or tools like `openapi-spec-validator`

#### 7.2 Try-It-Out Testing
- Test all endpoints through Swagger UI
- Verify authentication flow works
- Ensure request/response formats match implementation

#### 7.3 Integration Tests
- Add tests to verify OpenAPI spec generation
- Test that all documented endpoints exist
- Validate schema compatibility

### Phase 8: Documentation and Deployment

#### 8.1 Developer Documentation
- Update README with Swagger UI access instructions
- Document how to add OpenAPI annotations
- Create guidelines for maintaining API documentation

#### 8.2 Build Configuration
- Ensure OpenAPI spec is generated at compile time
- Configure for both development and production builds
- Consider generating static OpenAPI spec file for CI/CD

#### 8.3 Production Deployment
- Configure CORS for Swagger UI if needed
- Set appropriate server URLs in OpenAPI spec
- Consider CDN for Swagger UI assets

## File Structure

```
backend/
├── src/
│   ├── handlers/
│   │   ├── ledger.rs    # Add #[utoipa::path] to handlers
│   │   ├── account.rs
│   │   └── ...
│   ├── models/
│   │   ├── ledger.rs    # Add #[derive(ToSchema)] to models
│   │   ├── account.rs
│   │   └── ...
│   ├── openapi.rs       # OpenAPI configuration and spec generation
│   └── main.rs          # Mount Swagger UI route
```

## Configuration Options

### Environment Variables
```bash
# Control Swagger UI availability
ENABLE_SWAGGER_UI=true

# Set API base URL for production
API_BASE_URL=https://api.blackledger.io

# Authentication settings
AUTH_ENABLED=true
JWT_ISSUER=https://auth.blackledger.io
```

### Swagger UI Customization
- Custom CSS theming
- Logo and branding
- Default expansion settings
- Try-it-out enabled by default

## Security Considerations

1. **Production Environment**
   - Consider disabling Swagger UI in production
   - Or protect with additional authentication layer
   - Use environment variables to control availability

2. **Sensitive Information**
   - Don't expose internal implementation details
   - Sanitize example data
   - Be careful with error message details

3. **CORS Configuration**
   - Configure CORS appropriately for Swagger UI
   - Restrict origins in production

## Testing Strategy

1. **Unit Tests**
   - Test OpenAPI spec generation
   - Validate schema serialization

2. **Integration Tests**
   - Test all documented endpoints exist
   - Verify request/response formats
   - Test authentication flow

3. **Manual Testing**
   - Try all endpoints via Swagger UI
   - Test with different authentication states
   - Verify error handling

## Migration Path from Python

Since the Python FastAPI implementation likely has automatic OpenAPI generation:

1. Export OpenAPI spec from Python version
2. Use as reference for Rust implementation
3. Ensure compatibility in:
   - Endpoint paths
   - Request/response schemas
   - Authentication methods
   - Error response formats

## Success Criteria

- [ ] All API endpoints documented with OpenAPI annotations
- [ ] Swagger UI accessible at /swagger-ui
- [ ] Try-it-out functionality works for all endpoints
- [ ] Bearer token authentication functional in UI
- [ ] Request/response examples provided
- [ ] Search and filter parameters documented
- [ ] Error responses documented
- [ ] Production deployment considerations addressed
- [ ] 100% of handlers have OpenAPI documentation
- [ ] OpenAPI spec validates against OpenAPI 3.0 schema

## Timeline Estimate

- Phase 1-2: 1 day (Setup and core schemas)
- Phase 3: 2 days (Document all endpoints)
- Phase 4-5: 1 day (UI integration and auth)
- Phase 6: 1 day (Examples and advanced features)
- Phase 7-8: 1 day (Testing and deployment)

**Total: ~6 days for complete implementation**

## References

- [utoipa Documentation](https://docs.rs/utoipa/latest/utoipa/)
- [utoipa Examples](https://github.com/juhaku/utoipa/tree/master/examples)
- [OpenAPI 3.0 Specification](https://swagger.io/specification/)
- [Swagger UI Documentation](https://swagger.io/tools/swagger-ui/)