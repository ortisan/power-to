---
id: 0009-clean-architecture
title: Clean Architecture Boundaries
---

# Clean Architecture Boundaries

## Status

ACCEPTED on 2026-07-16 by explicit project-owner decision.

## Context

PowerTo must keep civic rules understandable and testable while HTTP, database,
identity, telemetry, and storage technologies evolve. Rust's module system can
enforce this separation, but only when dependency direction and composition
rules are explicit.

## Decision

Organize the backend according to Clean Architecture. Dependencies point
inward through four logical layers:

1. `domain` owns entities, value objects, policies, state transitions, and
   domain events. It has no knowledge of Axum, Diesel, OpenTelemetry, storage,
   identity providers, or serialization formats.
2. `application` owns use cases and declares inbound and outbound ports. It
   depends on `domain`, never on concrete adapters.
3. `adapters` implement outbound infrastructure ports and translate Diesel
   records, OIDC claims, media-provider operations, and notification payloads
   to and from application types.
4. `apps/api` owns the inbound Axum/HTTP adapter as well as the API composition
   root. `apps/worker` owns the worker composition root. Their executable entry
   points configure providers and wire concrete adapters to application ports.

These are logical boundaries inside the proposed modular monolith, not a
requirement for four network services or a crate for every domain module.

Domain events are business facts. Logs, spans, database rows, and queue
messages are representations created at outer boundaries. Observability wraps
use cases and adapters and is never required for a domain rule to execute.

## Consequences

### Positive

- Civic rules can be tested without a database, network, or telemetry runtime.
- Provider choices remain replaceable behind application ports.
- Framework types cannot silently become public contracts or domain entities.
- Dependency rules are enforceable with Cargo manifests and architecture tests.

### Negative

- Each boundary needs explicit mappings and error translation.
- Poorly designed generic repositories can hide useful database capabilities.
- The team must resist bypassing use cases for apparently simple CRUD changes.

### Neutral

- Clean Architecture does not imply microservices, event sourcing, or a
  framework-free composition root.
- PostgreSQL transactions may implement application unit-of-work ports without
  leaking Diesel types inward.

## Compliance

- `domain` depends on no other project layer.
- `application` depends only on `domain` and small framework-neutral libraries.
- `adapters` may depend inward; inner crates never depend back on adapters.
- Only composition roots select concrete adapters and read deployment config.
- Domain and application tests run without external services; adapter contract
  tests run against disposable real dependencies.
- Exceptions require an ADR with the affected dependency edge and removal plan.

## References

- [Architecture overview](../overview.md)
- [ADR 0006: Modular Monolith First](0006-modular-monolith.md)
