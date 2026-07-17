//! PowerTo application use cases and ports.
//!
//! Infrastructure adapters implement the ports defined here. Application code
//! may depend on the domain but never on adapters, frameworks, or providers.

pub mod health;
pub mod identity;
pub mod issues;
