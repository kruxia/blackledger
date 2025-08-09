# Dockerfile Optimization Plan for Minimal Image Size

## Current State Analysis

The current Dockerfile has a good multi-stage structure but can be optimized further:
- Base image: `debian:bookworm-slim` (~75MB)
- Binary size: ~8.6MB
- Total estimated image size: ~100MB+

### Current Issues
1. **Base image size**: Using `debian:bookworm-slim` when smaller alternatives exist
2. **Unnecessary dependencies**: Installing full `libpq5` when only client libs needed
3. **Build cache inefficiency**: Dependencies rebuild when only source changes
4. **Missing optimization flags**: Not using Rust's size optimization features
5. **Debug symbols**: Release build may still include debug info

## Optimization Approach

### 1. Use Alpine Linux or Distroless Images
- Replace `debian:bookworm-slim` with `alpine:3.19` (~7MB) or Google's distroless (~2MB)
- Alpine requires musl target compilation but results in much smaller images

### 2. Optimize Rust Compilation

Add to `Cargo.toml`:
```toml
[profile.release]
opt-level = "z"     # Optimize for size
lto = true          # Enable Link Time Optimization
codegen-units = 1   # Reduce parallel codegen for better optimization
strip = true        # Strip symbols
panic = "abort"     # Use abort instead of unwind

[profile.release-small]
inherits = "release"
opt-level = "s"     # Different size optimization level
```

### 3. Multi-stage Build Improvements
- Use cargo-chef for better dependency caching
- Separate SQLX offline mode preparation
- Use static linking where possible

### 4. Optimized Dockerfile Structure

Create `Dockerfile.minimal`:

```dockerfile
# =======================================================================
# Chef stage - prepare dependencies
FROM lukemathwalker/cargo-chef:latest-rust-1.88-alpine AS chef
WORKDIR /app
RUN apk add --no-cache musl-dev pkgconfig openssl-dev openssl-libs-static

# =======================================================================
# Planner stage - create recipe.json
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# =======================================================================
# Builder stage - build dependencies then app
FROM chef AS builder

# Copy recipe and build dependencies (cached)
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json

# Copy source and build application
COPY . .
ENV SQLX_OFFLINE=true

# Build with size optimizations
RUN cargo build --release --target x86_64-unknown-linux-musl --bin blackledger

# Strip the binary further
RUN strip /app/target/x86_64-unknown-linux-musl/release/blackledger

# =======================================================================
# Runtime stage - minimal final image
FROM alpine:3.19 AS runtime

# Install only essential runtime dependencies
RUN apk add --no-cache ca-certificates libgcc

# Create non-root user
RUN adduser -D -u 1000 blackledger

WORKDIR /app

# Copy only the binary and migrations
COPY --from=builder --chown=blackledger:blackledger \
    /app/target/x86_64-unknown-linux-musl/release/blackledger /usr/local/bin/
COPY --chown=blackledger:blackledger migrations /app/migrations

USER blackledger

ENV RUST_LOG=blackledger=info,tower_http=info,sqlx=warn
ENV PORT=8000

EXPOSE 8000

ENTRYPOINT ["/usr/local/bin/blackledger"]

# =======================================================================
# Alternative: Distroless image (even smaller)
FROM gcr.io/distroless/cc-debian12 AS runtime-distroless

COPY --from=builder /app/target/release/blackledger /usr/local/bin/
COPY migrations /app/migrations

WORKDIR /app
USER nonroot

ENV RUST_LOG=blackledger=info,tower_http=info,sqlx=warn
ENV PORT=8000

EXPOSE 8000

ENTRYPOINT ["/usr/local/bin/blackledger"]
```

### 5. Build Script for Minimal Size

Create `scripts/build-minimal.sh`:

```bash
#!/bin/bash
# Build with maximum size optimization
export CARGO_PROFILE_RELEASE_OPT_LEVEL=z
export CARGO_PROFILE_RELEASE_LTO=true
export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1
export CARGO_PROFILE_RELEASE_STRIP=true
export CARGO_PROFILE_RELEASE_PANIC=abort

# Build for musl target (static linking)
cargo build --release --target x86_64-unknown-linux-musl

# Additional stripping
strip target/x86_64-unknown-linux-musl/release/blackledger

# Build Docker image
docker build -f Dockerfile.minimal -t blackledger:minimal .

# Display size comparison
echo "Image sizes:"
docker images blackledger:minimal --format "table {{.Repository}}:{{.Tag}}\t{{.Size}}"
```

## Expected Results

1. **Base image reduction**: From ~75MB (Debian) to ~7MB (Alpine) or ~2MB (distroless)
2. **Binary size reduction**: From ~8.6MB to ~4-5MB with optimizations
3. **Total image size**: Target <15MB (from current ~100MB+)
4. **Build time improvement**: Better caching with cargo-chef
5. **Security improvement**: Minimal attack surface with distroless

## Trade-offs

1. **Compilation time**: Increased due to optimizations (LTO adds significant time)
2. **Debugging**: Harder without symbols (keep debug builds for development)
3. **Compatibility**: Alpine/musl may have edge cases with some crates
4. **Performance**: Size optimizations may slightly reduce runtime performance (~5-10%)

## Implementation Steps

1. **Phase 1**: Add Cargo.toml optimization profile
2. **Phase 2**: Create and test Alpine-based Dockerfile
3. **Phase 3**: Benchmark performance impact
4. **Phase 4**: Test distroless option if Alpine successful
5. **Phase 5**: Update CI/CD pipelines

## Testing Checklist

- [ ] Database connections work correctly
- [ ] TLS/HTTPS functionality verified
- [ ] JWT validation works
- [ ] All migrations run successfully
- [ ] Performance regression tests pass
- [ ] Memory usage is acceptable
- [ ] Container starts and stops cleanly
- [ ] Signals handled correctly (SIGTERM)

## Recommendation

Start with the Alpine-based approach as it provides a good balance of size reduction and compatibility. Test thoroughly, especially:
- Database connection pooling
- TLS functionality with external services
- Time zone handling (Alpine has different tzdata)

Keep the current Dockerfile as `Dockerfile.standard` for development/debugging needs.

## Metrics to Track

- Image size (docker images)
- Container startup time
- Memory usage at idle
- Memory usage under load
- Request latency p50/p95/p99
- Build time comparison

## Alternative Approaches (Not Recommended)

1. **scratch image**: Too minimal, lacks basic tools
2. **busybox**: Limited compatibility with Rust binaries
3. **Custom minimal base**: Maintenance overhead
4. **UPX compression**: Can cause runtime issues

## References

- [cargo-chef Documentation](https://github.com/LukeMathWalker/cargo-chef)
- [Distroless Images](https://github.com/GoogleContainerTools/distroless)
- [Rust Binary Size Optimization](https://github.com/johnthagen/min-sized-rust)
- [Alpine Linux Package Database](https://pkgs.alpinelinux.org/packages)