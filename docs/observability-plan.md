# Blackledger Observability Implementation Plan

## Overview

This document outlines the implementation plan for comprehensive observability in the Blackledger Rust backend, covering OpenTelemetry integration, distributed tracing, metrics collection, performance profiling, and audit logging as specified in the ROADMAP (Priority 4).

## Goals

- **Distributed Tracing**: Track requests across the entire system with correlation IDs
- **Metrics Collection**: Export key performance indicators to Prometheus
- **Performance Profiling**: Identify bottlenecks and optimize critical paths
- **Audit Logging**: Track all state-changing operations for compliance
- **Error Tracking**: Capture and categorize errors with context

## Architecture

### 1. OpenTelemetry Integration

#### Dependencies to Add

```toml
# Cargo.toml additions
[dependencies]
# OpenTelemetry Core
opentelemetry = "0.27"
opentelemetry_sdk = { version = "0.27", features = ["rt-tokio"] }
opentelemetry-semantic-conventions = "0.27"

# Tracing Integration
tracing-opentelemetry = "0.28"
opentelemetry-otlp = { version = "0.27", features = ["tonic", "metrics"] }

# Metrics
opentelemetry-prometheus = "0.27"
prometheus = "0.13"

# Additional tracing features
tracing-subscriber = { version = "0.3", features = [
    "env-filter",
    "json",
    "registry",
    "tracing-log"
] }
tracing-appender = "0.2"

# Performance profiling
pprof = { version = "0.14", features = ["flamegraph", "criterion"] }
```

#### Core Module Structure

```
src/
├── observability/
│   ├── mod.rs           # Module exports and configuration
│   ├── tracing.rs       # Distributed tracing setup
│   ├── metrics.rs       # Prometheus metrics definitions
│   ├── audit.rs         # Audit logging implementation
│   └── profiling.rs     # Performance profiling utilities
```

### 2. Distributed Tracing Implementation

#### Tracing Setup (`src/observability/tracing.rs`)

```rust
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    propagation::TraceContextPropagator,
    runtime,
    trace::{self, RandomIdGenerator, Sampler},
    Resource,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};

pub fn init_tracing(service_name: &str, otlp_endpoint: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Set global propagator for W3C Trace Context
    global::set_text_map_propagator(TraceContextPropagator::new());
    
    // Configure OTLP exporter
    let otlp_exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(otlp_endpoint);
    
    // Create tracer
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(otlp_exporter)
        .with_trace_config(
            trace::Config::default()
                .with_sampler(Sampler::AlwaysOn)
                .with_id_generator(RandomIdGenerator::default())
                .with_resource(Resource::new(vec![
                    KeyValue::new("service.name", service_name.to_string()),
                    KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
                ])),
        )
        .install_batch(runtime::Tokio)?;
    
    // Create telemetry layer
    let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    
    // Initialize subscriber with telemetry layer
    let subscriber = Registry::default()
        .with(EnvFilter::from_default_env())
        .with(telemetry_layer)
        .with(tracing_subscriber::fmt::layer().json());
    
    tracing::subscriber::set_global_default(subscriber)?;
    
    Ok(())
}
```

#### Request Tracing Middleware

```rust
use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};
use opentelemetry::trace::{SpanKind, TraceContextExt, Tracer};
use tracing::{Instrument, Span};

pub async fn trace_layer(req: Request, next: Next) -> Response {
    let method = req.method().clone();
    let uri = req.uri().path().to_string();
    
    // Extract trace context from headers
    let parent_cx = extract_trace_context(req.headers());
    
    // Create span for this request
    let span = tracing::span!(
        tracing::Level::INFO,
        "http_request",
        otel.kind = ?SpanKind::Server,
        http.method = %method,
        http.target = %uri,
        http.status_code = tracing::field::Empty,
        trace_id = tracing::field::Empty,
    );
    
    // Record trace ID
    let trace_id = span.context().span().span_context().trace_id();
    span.record("trace_id", trace_id.to_string());
    
    // Execute request with span
    let response = next.run(req).instrument(span.clone()).await;
    
    // Record response status
    span.record("http.status_code", response.status().as_u16());
    
    response
}
```

### 3. Metrics Collection

#### Metrics Definitions (`src/observability/metrics.rs`)

```rust
use once_cell::sync::Lazy;
use prometheus::{
    register_counter_vec, register_histogram_vec, register_gauge_vec,
    CounterVec, HistogramVec, GaugeVec, Registry,
};

// HTTP Metrics
pub static HTTP_REQUESTS_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "http_requests_total",
        "Total number of HTTP requests",
        &["method", "endpoint", "status"]
    ).unwrap()
});

pub static HTTP_REQUEST_DURATION: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "http_request_duration_seconds",
        "HTTP request latency",
        &["method", "endpoint", "status"],
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0]
    ).unwrap()
});

// Database Metrics
pub static DB_QUERY_DURATION: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "db_query_duration_seconds",
        "Database query execution time",
        &["query_type", "table"],
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0]
    ).unwrap()
});

pub static DB_CONNECTIONS_ACTIVE: Lazy<GaugeVec> = Lazy::new(|| {
    register_gauge_vec!(
        "db_connections_active",
        "Number of active database connections",
        &["pool"]
    ).unwrap()
});

// Business Metrics
pub static TRANSACTIONS_POSTED: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "transactions_posted_total",
        "Total number of transactions posted",
        &["ledger_id", "status"]
    ).unwrap()
});

pub static TRANSACTION_AMOUNT: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "transaction_amount",
        "Distribution of transaction amounts",
        &["ledger_id", "currency"],
        vec![1.0, 10.0, 100.0, 1000.0, 10000.0, 100000.0, 1000000.0]
    ).unwrap()
});

pub static ACCOUNT_BALANCE: Lazy<GaugeVec> = Lazy::new(|| {
    register_gauge_vec!(
        "account_balance",
        "Current account balances",
        &["ledger_id", "account_id", "currency"]
    ).unwrap()
});

// Performance Metrics
pub static VALIDATION_DURATION: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "validation_duration_seconds",
        "Time spent in validation logic",
        &["validation_type"],
        vec![0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1]
    ).unwrap()
});
```

#### Metrics Middleware

```rust
use std::time::Instant;
use axum::{extract::Request, middleware::Next, response::Response};

pub async fn metrics_layer(req: Request, next: Next) -> Response {
    let start = Instant::now();
    let method = req.method().to_string();
    let path = req.uri().path().to_string();
    
    let response = next.run(req).await;
    
    let duration = start.elapsed().as_secs_f64();
    let status = response.status().as_str().to_string();
    
    // Record metrics
    HTTP_REQUESTS_TOTAL
        .with_label_values(&[&method, &path, &status])
        .inc();
    
    HTTP_REQUEST_DURATION
        .with_label_values(&[&method, &path, &status])
        .observe(duration);
    
    response
}
```

### 4. Audit Logging

#### Audit Log Model (`src/observability/audit.rs`)

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub trace_id: String,
    pub user_id: Option<String>,
    pub client_ip: String,
    pub method: String,
    pub path: String,
    pub action: AuditAction,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub changes: Option<serde_json::Value>,
    pub result: AuditResult,
    pub error_message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum AuditAction {
    Create,
    Update,
    Delete,
    Read,
    Post,  // For transaction posting
    Void,  // For transaction voiding
}

#[derive(Debug, Serialize, Deserialize)]
pub enum AuditResult {
    Success,
    Failure,
    ValidationError,
    AuthorizationError,
}

impl AuditLog {
    pub async fn record(
        pool: &PgPool,
        log: AuditLog,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            INSERT INTO audit_logs (
                id, timestamp, trace_id, user_id, client_ip,
                method, path, action, resource_type, resource_id,
                changes, result, error_message
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            "#,
            log.id,
            log.timestamp,
            log.trace_id,
            log.user_id,
            log.client_ip,
            log.method,
            log.path,
            serde_json::to_value(&log.action).unwrap(),
            log.resource_type,
            log.resource_id,
            log.changes,
            serde_json::to_value(&log.result).unwrap(),
            log.error_message,
        )
        .execute(pool)
        .await?;
        
        Ok(())
    }
}
```

#### Audit Middleware Integration

```rust
use axum::{
    extract::{ConnectInfo, State},
    middleware::Next,
    response::Response,
};
use std::net::SocketAddr;

pub async fn audit_layer(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(pool): State<PgPool>,
    req: Request,
    next: Next,
) -> Response {
    let method = req.method().to_string();
    let path = req.uri().path().to_string();
    let trace_id = get_trace_id(&req);
    let user_id = extract_user_id(&req);
    
    // Determine if this is an auditable action
    let should_audit = matches!(
        req.method(),
        &Method::POST | &Method::PUT | &Method::PATCH | &Method::DELETE
    );
    
    let response = next.run(req).await;
    
    if should_audit {
        let audit_log = AuditLog {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            trace_id,
            user_id,
            client_ip: addr.ip().to_string(),
            method,
            path: path.clone(),
            action: determine_action(&method, &path),
            resource_type: extract_resource_type(&path),
            resource_id: extract_resource_id(&path),
            changes: None, // Would need to capture request body
            result: if response.status().is_success() {
                AuditResult::Success
            } else {
                AuditResult::Failure
            },
            error_message: None,
        };
        
        // Fire and forget audit logging
        tokio::spawn(async move {
            let _ = AuditLog::record(&pool, audit_log).await;
        });
    }
    
    response
}
```

### 5. Performance Profiling

#### CPU Profiling Integration (`src/observability/profiling.rs`)

```rust
use pprof::ProfilerGuard;
use std::fs::File;
use std::io::Write;

pub struct CpuProfiler {
    guard: Option<ProfilerGuard<'static>>,
}

impl CpuProfiler {
    pub fn new(frequency: i32) -> Self {
        Self {
            guard: Some(ProfilerGuard::new(frequency).unwrap()),
        }
    }
    
    pub fn report(&mut self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(guard) = self.guard.take() {
            let report = guard.report().build()?;
            let mut file = File::create(path)?;
            report.flamegraph(&mut file)?;
        }
        Ok(())
    }
}

// Profiling endpoint
pub async fn profile_handler() -> impl IntoResponse {
    let mut profiler = CpuProfiler::new(100);
    
    // Profile for 30 seconds
    tokio::time::sleep(Duration::from_secs(30)).await;
    
    let mut buffer = Vec::new();
    profiler.report_to_buffer(&mut buffer).unwrap();
    
    Response::builder()
        .header("Content-Type", "image/svg+xml")
        .body(buffer)
        .unwrap()
}
```

#### Memory Profiling

```rust
use jemalloc_ctl::{stats, epoch};

pub async fn memory_stats() -> impl IntoResponse {
    // Update statistics
    epoch::advance().unwrap();
    
    let allocated = stats::allocated::read().unwrap();
    let resident = stats::resident::read().unwrap();
    
    Json(json!({
        "allocated_bytes": allocated,
        "resident_bytes": resident,
        "allocated_mb": allocated as f64 / 1_048_576.0,
        "resident_mb": resident as f64 / 1_048_576.0,
    }))
}
```

## Implementation Plan

### Phase 1: Core Infrastructure (Days 1-2)

1. Add OpenTelemetry dependencies to Cargo.toml
2. Create `src/observability` module structure
3. Implement basic tracing setup with OTLP exporter
4. Add trace context propagation middleware
5. Configure environment-based initialization

### Phase 2: Distributed Tracing (Days 2-3)

1. Instrument all HTTP endpoints with spans
2. Add database query tracing with SQLx hooks
3. Implement trace context injection in service layer
4. Add correlation IDs to all log messages
5. Create custom spans for business operations (posting, validation)

### Phase 3: Metrics Collection (Days 3-4)

1. Define Prometheus metrics for all key indicators
2. Implement metrics middleware for HTTP requests
3. Add database connection pool metrics
4. Create business metrics for transactions and accounts
5. Set up `/metrics` endpoint for Prometheus scraping

### Phase 4: Audit Logging (Days 4-5)

1. Create audit log database schema
2. Implement audit log model and storage
3. Add audit middleware for state-changing operations
4. Include user context and request details
5. Create audit log query endpoints for compliance

### Phase 5: Performance Profiling (Days 5-6)

1. Integrate pprof for CPU profiling
2. Add memory profiling with jemalloc
3. Create profiling endpoints (protected)
4. Implement automatic profiling triggers
5. Add performance benchmarks with criterion

### Phase 6: Integration & Testing (Days 6-7)

1. Configure Docker Compose with observability stack:
   - Jaeger for trace visualization
   - Prometheus for metrics storage
   - Grafana for dashboards
2. Create Grafana dashboards for key metrics
3. Add integration tests for observability features
4. Document configuration and deployment
5. Performance testing with observability enabled

## Configuration

### Environment Variables

```bash
# OpenTelemetry Configuration
OTEL_SERVICE_NAME=blackledger
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
OTEL_TRACES_SAMPLER=always_on
OTEL_TRACES_SAMPLER_ARG=1.0

# Metrics Configuration
METRICS_ENABLED=true
METRICS_PORT=9090
METRICS_PATH=/metrics

# Audit Logging
AUDIT_ENABLED=true
AUDIT_LOG_LEVEL=info
AUDIT_RETENTION_DAYS=90

# Profiling (only in debug/staging)
PROFILING_ENABLED=false
PROFILING_CPU_FREQUENCY=100
PROFILING_AUTH_TOKEN=secret_token
```

### Docker Compose Addition

```yaml
# docker-compose.observability.yml
services:
  jaeger:
    image: jaegertracing/all-in-one:latest
    ports:
      - "16686:16686"  # Jaeger UI
      - "4317:4317"    # OTLP gRPC
      - "4318:4318"    # OTLP HTTP
    environment:
      - COLLECTOR_OTLP_ENABLED=true

  prometheus:
    image: prom/prometheus:latest
    ports:
      - "9091:9090"
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
      - prometheus_data:/prometheus
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'

  grafana:
    image: grafana/grafana:latest
    ports:
      - "3000:3000"
    volumes:
      - grafana_data:/var/lib/grafana
      - ./grafana/dashboards:/etc/grafana/provisioning/dashboards
      - ./grafana/datasources:/etc/grafana/provisioning/datasources
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
      - GF_USERS_ALLOW_SIGN_UP=false

volumes:
  prometheus_data:
  grafana_data:
```

## Monitoring Dashboards

### Key Metrics to Display

1. **System Health Dashboard**
   - Request rate and latency (P50, P95, P99)
   - Error rate by endpoint
   - Active database connections
   - Memory and CPU usage

2. **Business Metrics Dashboard**
   - Transactions posted per minute
   - Transaction amounts by currency
   - Account balance changes
   - Validation error rates

3. **Performance Dashboard**
   - Query execution times
   - Slow query analysis
   - Cache hit rates
   - Request queue depth

4. **Audit Dashboard**
   - Failed authentication attempts
   - State changes by user
   - Error patterns
   - Compliance events

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_trace_context_propagation() {
        // Test that trace context is properly propagated
    }
    
    #[tokio::test]
    async fn test_metrics_recording() {
        // Test that metrics are recorded correctly
    }
    
    #[tokio::test]
    async fn test_audit_log_creation() {
        // Test audit log generation
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_end_to_end_tracing() {
    // Start test OTLP collector
    // Make API request
    // Verify trace was exported
}

#[tokio::test]
async fn test_metrics_endpoint() {
    // Make requests
    // Scrape metrics endpoint
    // Verify metrics presence and values
}
```

## Performance Impact

Expected performance overhead:
- **Tracing**: ~2-5% with sampling at 1.0
- **Metrics**: <1% for counter/histogram updates
- **Audit Logging**: ~3-5% for async database writes
- **Total Expected Overhead**: 5-10% in worst case

Mitigation strategies:
- Use sampling for high-volume endpoints
- Batch audit log writes
- Use async/fire-and-forget for non-critical observability
- Implement circuit breakers for observability backends

## Security Considerations

1. **Sensitive Data**: Ensure no sensitive data in traces/logs
2. **Access Control**: Protect profiling and metrics endpoints
3. **Data Retention**: Implement automatic cleanup policies
4. **Compliance**: Audit logs must be tamper-proof
5. **Performance**: Rate limit observability data to prevent DoS

## Rollout Strategy

1. **Development Environment**: Full observability stack
2. **Staging Environment**: Production-like configuration
3. **Production Rollout**:
   - Start with 10% sampling
   - Monitor performance impact
   - Gradually increase to 100% sampling
   - Enable profiling only when debugging

## Success Criteria

- ✅ All API requests have trace IDs
- ✅ P99 latency visible in real-time
- ✅ Database query performance tracked
- ✅ All state changes audited
- ✅ Performance overhead <10%
- ✅ Mean time to detection <5 minutes
- ✅ Root cause analysis time reduced by 50%

## Future Enhancements

1. **Distributed Tracing**: Extend to message queues and external services
2. **Custom Dashboards**: Business-specific KPI dashboards
3. **Alerting**: Automated alerts based on SLOs
4. **Cost Optimization**: Intelligent sampling based on endpoint importance
5. **AI/ML Integration**: Anomaly detection and predictive alerts