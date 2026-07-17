//! Infrastructure implementations for PowerTo application ports.
//!
//! Diesel models, provider SDK types, and OpenTelemetry setup stay in this
//! outer layer and must not leak into the domain.

pub mod account_directory;
pub mod database;
pub mod issue_store;
pub mod object_storage;
pub mod observability;
pub mod oidc;
