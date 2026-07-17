---
id: 0004-rust-backend
title: Rust for the Backend
---

# Rust for the Backend

## Status

ACCEPTED on 2026-07-16 by explicit project-owner decision.

## Context

PowerTo will enforce geographic eligibility, concurrent voting constraints,
privileged workflow transitions, and tamper-evident audit behavior. These are
long-lived correctness and security concerns. The repository did not yet
contain an application or an accepted backend language.

## Decision

The PowerTo API, background worker, domain rules, and first-party backend
integrations will be implemented in stable Rust using Rust edition 2024.

The stable toolchain is pinned in `rust-toolchain.toml` and updated through a
reviewed change that runs the full test suite. A non-Rust backend service
requires a new ADR showing a capability unavailable through a Rust library,
process isolation, or external standards-based service.

This decision does not require Rust in the browser, documentation site,
declarative infrastructure, SQL migrations, or third-party systems.

## Consequences

### Positive

- Domain invariants and state transitions benefit from Rust's type system.
- Memory safety reduces an important class of service vulnerabilities.
- API and worker can share domain and application crates without duplicating
  rules.
- A single compiled artifact has predictable runtime dependencies and resource
  use.

### Negative

- Rust has a steeper onboarding curve than the most common web languages.
- Compile times and generic error messages require deliberate dependency and
  module design.
- Some civic, identity, and geospatial integrations may have a smaller Rust
  ecosystem and require narrow adapters or standards-based external services.
- Hyperledger Fabric has no official Rust application or chaincode API, so its
  proposed design conflicts with an all-Rust backend.

### Neutral

- Frontend and documentation technology remain independent decisions.
- Rust improves implementation safety but does not by itself provide ballot
  secrecy, Sybil resistance, legal legitimacy, or secure operations.

## Compliance

- Backend code lives in a Cargo workspace and compiles on the pinned stable
  toolchain.
- CI runs `cargo fmt --check`, Clippy with warnings denied, unit and integration
  tests, and dependency/security checks.
- Domain crates do not import HTTP, database, identity-provider, or object-store
  frameworks.
- New first-party backend processes written in another language fail
  architecture review unless a superseding ADR is accepted.

## References

- [Rust release announcements](https://blog.rust-lang.org/releases/)
- [Rust 2024 Edition Guide](https://doc.rust-lang.org/edition-guide/)
