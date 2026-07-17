---
id: 0006-modular-monolith
title: Modular Monolith First
---

# Modular Monolith First

## Status

PROPOSED

## Context

PowerTo spans issue reporting, moderation, identity and geographic eligibility,
voting, prioritization, future procurement, delivery, evaluation, and audit.
These are meaningful domain boundaries, but the product has no production load,
stable organizational ownership, or implemented application. Starting with
distributed services would add network failure, distributed consistency, and
operational work before those boundaries are validated.

## Decision

Build one modular Rust backend codebase with two initial processes: a stateless
HTTP API and a background worker. Both use one PostgreSQL/PostGIS source of
truth and share framework-free domain/application modules.

Modules own their state and behavior. Cross-module work happens through use
cases, immutable domain events, and versioned read models. Reliable background
work uses a transactional outbox.

Start with a small number of Cargo crates (`domain`, `application`, `adapters`,
and composition binaries) and vertical modules within the inner crates. Extract
a module to a separate crate or service only after its boundary and need are
demonstrated. ADR 0009 defines the dependency direction between these layers.

## Consequences

### Positive

- Civic state, audit events, and outbox work can commit atomically.
- Local development, testing, deployment, and incident response remain small.
- Rust modules and crate dependency direction still enforce design boundaries.
- A future service extraction has seams through ports and outbox events.

### Negative

- A bad module boundary can still create a coupled monolith.
- All modules initially share a release cadence and database availability.
- Hot modules scale with the whole API until they are extracted.

### Neutral

- This does not mean a single undifferentiated crate or table ownership by
  convention alone.
- The web application and OIDC provider remain separately deployable systems.

## Compliance

- `domain` never depends on `application`, `adapters`, Axum, Diesel, or an
  external provider; `application` never depends on `adapters`.
- Each context has explicit public entry points and does not query another
  context's private tables.
- API and worker composition roots wire ports to adapters.
- A new independently deployed service, broker, or database requires an ADR
  with measured scaling, security, ownership, or availability evidence.

## References

- [ADR 0009: Clean Architecture Boundaries](0009-clean-architecture.md)
