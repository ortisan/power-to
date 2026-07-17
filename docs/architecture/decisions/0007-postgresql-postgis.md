---
id: 0007-postgresql-postgis
title: PostgreSQL and PostGIS as the System of Record
---

# PostgreSQL and PostGIS as the System of Record

## Status

PROPOSED

## Context

PowerTo needs ACID workflow transitions, concurrency-safe voting constraints,
spatial matching between issues and jurisdictions, proximity rules, audit
records, and reliable job publication. Using separate databases for these
concerns would introduce consistency problems during the earliest product
stage.

## Decision

Use PostgreSQL as the initial system of record and PostGIS for all authoritative
spatial data and queries. Use separate logical schemas and database roles for
public civic data, private identity/location data, audit data, and jobs.

Use spatial indexes and index-aware predicates. Store a precise private
location and a separately derived public/generalized location when both are
required. Do not put evidence binaries in PostgreSQL; store object metadata and
integrity hashes only.

PostgreSQL also hosts the transactional outbox. It does not become an unbounded
analytics warehouse or a substitute for object storage.

## Consequences

### Positive

- One transaction protects civic state, audit evidence, and asynchronous work.
- PostGIS directly supports jurisdiction boundaries and proximity queries.
- Constraints and indexes provide a second line of correctness behind Rust.
- Managed PostgreSQL/PostGIS is available from multiple vendors.

### Negative

- Database availability initially affects every backend module.
- Spatial schema, coordinate systems, and query plans require specialist care.
- Sensitive and public data sharing one physical service requires strict roles,
  backups, projections, and access review.

### Neutral

- Read replicas, partitioning, search indexes, or a warehouse may be added from
  measured needs without changing the transactional source of truth.

## Compliance

- Migrations enable PostGIS explicitly and declare SRIDs and GiST indexes.
- Integration tests use real PostgreSQL/PostGIS, not SQLite substitutes.
- CI exercises uniqueness races, transaction rollback, and representative
  spatial query plans.
- Public API tests prove precise location and private identity fields cannot be
  selected through public projections.
- A second database or cache that owns authoritative civic state requires a
  superseding ADR.

## References

- [PostgreSQL documentation](https://www.postgresql.org/docs/current/)
- [PostGIS spatial index guidance](https://postgis.net/documentation/faq/spatial-indexes/)
- [`postgis_diesel` types and functions](https://docs.rs/postgis_diesel/latest/postgis_diesel/)
