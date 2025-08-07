//! # Blackledger - Double-Entry Accounting System
//!
//! Blackledger is a REST API for double-entry accounting with immutable transactions,
//! multi-currency support, and optimistic locking for concurrent operations.
//!
//! ## Key Features
//!
//! - **Immutable Transactions**: Once posted, transactions cannot be modified or deleted
//! - **Double-Entry Accounting**: All transactions must balance (debits = credits)
//! - **Multi-Currency**: Each entry specifies its currency code
//! - **Decimal Precision**: Uses `rust_decimal` for accurate financial calculations
//! - **Optimistic Locking**: Account versioning prevents concurrent modifications
//!
//! ## Architecture
//!
//! The system is organized into several modules:
//!
//! - [`api`]: HTTP handlers and routing
//! - [`auth`]: JWT authentication and authorization
//! - [`config`]: Application configuration
//! - [`db`]: Database connection and queries
//! - [`error`]: Error types and handling
//! - [`models`]: Domain models (ledger, account, transaction, entry)
//! - [`services`]: Business logic (validation, posting)

/// API layer with HTTP handlers, routing, pagination, and search
pub mod api;

/// Authentication and authorization with JWT support
pub mod auth;

/// Application configuration from environment variables
pub mod config;

/// Database layer with connection pooling and query functions
pub mod db;

/// Error types and HTTP response mapping
pub mod error;

/// Domain models for accounting entities
pub mod models;

/// Business logic services for validation and transaction posting
pub mod services;
