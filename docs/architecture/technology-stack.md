---
id: technology-stack
title: Technology Stack
sidebar_label: Technology Stack
---

# Technology Stack

This document is the technology baseline as of **2026-07-16**. Implemented
backend versions are pinned by `backend/Cargo.toml`, `backend/Cargo.lock`, and
the local deployment manifests, then updated through small, tested changes. A
major version is never adopted merely because it is the newest one.

The executable foundation currently proves the Clean Architecture dependency
direction, an Axum health/OpenAPI surface, Diesel-async PostgreSQL readiness,
Atlas bootstrap, OTLP export, and Victoria ingestion. It does not yet prove a
civic product vertical slice, spatial Diesel mappings, identity, or concrete
R2/S3/GCS SDK adapters; their proposed status remains meaningful.

`ACCEPTED` means the project owner has made the decision. `PROPOSED` means the
choice is the current architectural recommendation and should be validated in
the first vertical slice.

## Decision summary

| Area | Selection | Status | Why |
| --- | --- | --- | --- |
| Architecture boundaries | Clean Architecture inside a modular backend | **ACCEPTED** | Explicit project decision; domain and use cases stay independent of frameworks and providers |
| Backend language | Rust stable 1.97, edition 2024 | **ACCEPTED** | Explicit project decision; strong type and memory safety for trust-sensitive workflows |
| Persistence library | Diesel 2.3 | **ACCEPTED** | Explicit project decision; typed query builder and compile-time schema mapping |
| Database migrations | Atlas versioned SQL workflow | **ACCEPTED** | Explicit project decision; reviewed SQL, migration integrity, CI validation, and release-job apply |
| HTTP framework | Axum 0.8 on Tokio 1 and Tower | PROPOSED | Small abstraction layer, composable middleware, shared Tokio ecosystem, and easy testable routers |
| Async database access | `diesel-async` 0.9 with its `bb8` pool | PROPOSED | Keeps Diesel's query model without blocking Tokio request threads |
| Primary database | PostgreSQL 18 | PROPOSED | Transactions, constraints, JSON when justified, mature operations, and extension support |
| Spatial data | PostGIS 3.6 with `postgis_diesel` 3.1 | PROPOSED, spike required | Jurisdiction boundaries, proximity, generalized public locations, GiST indexes, and Diesel mappings |
| API contract | REST/JSON, OpenAPI 3.1 with Utoipa 5 | PROPOSED | Public, language-neutral contract and generated clients without GraphQL operational complexity |
| Serialization | Serde 1 and `serde_json` | PROPOSED | Rust ecosystem standard; API DTOs remain separate from domain and Diesel models |
| Authentication | OpenID Connect Authorization Code flow; Keycloak as local/reference provider | PROPOSED | Standards-based federation and a path to government identity without embedding passwords in PowerTo |
| Web application | Next.js App Router, React, TypeScript, Node.js 24 LTS | PROPOSED | Accessible server-rendered public pages, responsive authenticated UI, and a mature web ecosystem |
| Mobile product clients | Android and iOS apps for media/location capture, bounded geofencing, offline work, and road surveys | **ACCEPTED** | Explicit project decision; on-site evidence and motion sensing require installed mobile clients |
| Mobile implementation | KMP shared core with Jetpack Compose on Android and SwiftUI on iOS | PROPOSED, physical-device spike required | Shares domain/data/sync logic while keeping camera, sensors, geofence, and background behavior native |
| Mobile shared data | Ktor client, Kotlin serialization/coroutines, and Room KMP | PROPOSED | Multiplatform API/offline outbox with explicit local schema migrations and native file storage for large evidence |
| Map rendering | MapLibre GL JS on web and MapLibre Native on Android/iOS | PROPOSED | Open-source, provider-neutral rendering for issues, affected areas, and road-quality segments |
| Accessible UI primitives | React Aria Components plus project-owned design tokens | PROPOSED | Keyboard, screen-reader, touch, internationalization, and high-contrast behavior without adopting a fixed visual identity |
| Styling | CSS variables and Tailwind CSS utilities | PROPOSED | Consistent tokens and fast responsive implementation; semantic components hide utility details |
| User media storage | Clean Architecture port with Cloudflare R2, AWS S3, and Google Cloud Storage adapters | **ACCEPTED** | Explicit project decision; provider choice by configuration with safe direct uploads and portable locators |
| Background work | PostgreSQL transactional outbox processed by the Rust worker | PROPOSED | Reliable work with one source of truth and no broker in the MVP |
| Observability | `tracing` + OpenTelemetry/OTLP Collector + VictoriaMetrics, VictoriaLogs, and VictoriaTraces | **ACCEPTED** | Explicit project decision; vendor-neutral instrumentation with open-source Victoria backends |
| Local development | Docker Compose for PostgreSQL/PostGIS, OIDC, telemetry, and storage emulators | PROPOSED | Reproducible dependencies while Rust and Node processes retain fast local feedback; real provider contract tests remain required |
| CI | GitHub Actions, Rust checks, Atlas migration validation, frontend checks, and dependency audit | PROPOSED | Matches the repository and keeps quality gates reviewable in public |
| Deployment | OCI containers on a managed container platform | PROPOSED | Portable, simple operations; no Kubernetes until scaling or isolation evidence exists |

## Rust backend

Use the stable toolchain pinned by `rust-toolchain.toml`, Rust edition 2024,
and a Cargo workspace. The initial dependency baseline is:

- [Axum](https://docs.rs/axum/latest/axum/) for routing, extractors, and HTTP
  handlers
- [Tokio](https://tokio.rs/tokio/tutorial) as the asynchronous runtime
- [Tower HTTP](https://docs.rs/tower-http/latest/tower_http/) for request IDs,
  limits, timeouts, sensitive headers, CORS where needed, and request tracing
- [Serde](https://serde.rs/) for DTO serialization and deserialization
- [Utoipa](https://docs.rs/utoipa/latest/utoipa/) and `utoipa-axum` for OpenAPI
  3.1
- `thiserror` for typed library/application errors; `anyhow` only at binary and
  job boundaries where recovery is not domain-specific
- `tracing` and `tracing-subscriber` for structured spans and events
- `uuid` with UUIDv7 for database-friendly internal identifiers; opaque public
  references may use a different representation where timestamp disclosure is
  undesirable

Axum is preferred over Actix Web because the application needs composable Tower
middleware and architectural transparency more than the last increment of
framework benchmark throughput. Actix Web remains a viable fallback if a spike
reveals a concrete Axum limitation.

## Diesel persistence

Diesel is confined to outer persistence adapters. Domain entities do not derive
`Queryable`, `Insertable`, or `AsChangeset`.

Use:

- [Diesel 2.3](https://docs.rs/diesel/latest/diesel/) with PostgreSQL, UUID,
  time, and JSON features enabled only when used
- [`diesel-async`](https://docs.rs/diesel-async/latest/diesel_async/) with
  `AsyncPgConnection` and the `bb8` pool so database I/O does not block Tokio
- Atlas versioned SQL migrations as the physical schema history
- Generated `schema.rs` checked into version control, with CI failing when it
  differs from `diesel print-schema` after Atlas migrates a disposable database
- Explicit repository mappings between persistence records and domain types

The initial manifest does not enable Diesel's synchronous `postgres` connection
feature. `diesel-async/postgres` enables the PostgreSQL backend types and uses
`tokio-postgres`, avoiding an unused `libpq` dependency. If a later operation
requires synchronous Diesel, it must be isolated in a bounded blocking adapter
and justify the additional native runtime dependency.

Production migrations run through Atlas as a separate release step with a
dedicated database role. API replicas do not race to migrate on startup.
Destructive migrations use an expand/backfill/verify/contract sequence and
include a roll-forward or restore plan.

Diesel was chosen instead of SQLx by explicit project decision. SeaORM and raw
`tokio-postgres` are not part of the initial stack.

## Atlas migrations

Atlas is the sole migration engine. Keep `atlas.hcl`, timestamped SQL files,
and `atlas.sum` under `backend/db`. Use the versioned workflow: author or
generate a migration, review the SQL, validate it on disposable
PostgreSQL/PostGIS, and apply the same immutable directory through a serialized
release job.

At minimum, CI validates the directory checksum and SQL, replays the complete
history into an empty database, runs repository tests, and checks the generated
Diesel `schema.rs`. `atlas migrate lint` is used only if the pinned Atlas
edition supports the required checks. The design does not depend on Atlas
Cloud or paid features; PostGIS and advanced-object behavior requires a spike
because Atlas is open-core and some capabilities differ by edition.

Migration files use Atlas's transaction-per-file default. A non-transactional
operation such as `CREATE INDEX CONCURRENTLY` is isolated and explicitly marked,
with a failure and retry procedure. Applied files are never rewritten and
`atlas migrate hash` is not used to conceal drift.

See [ADR 0012](decisions/0012-atlas-database-migrations.md).

## PostgreSQL and PostGIS

PostgreSQL is the transactional boundary for issue state, effective votes,
audit records, and outbox messages. PostGIS is required because geographic
eligibility is a core domain rule, not a presentation feature.

The initial spatial conventions are:

- WGS 84 (`SRID 4326`) at API boundaries
- `geography(Point, 4326)` where distances must be interpreted in meters
- `geometry(MultiPolygon, 4326)` for administrative boundaries
- GiST indexes on spatial columns
- Index-aware predicates such as `ST_DWithin`, `ST_Intersects`, and
  `ST_Covers`
- A separately stored public/generalized location; never round a private point
  at response-serialization time

[`postgis_diesel`](https://docs.rs/postgis_diesel/latest/postgis_diesel/)
currently exposes Diesel `Geometry` and `Geography` SQL types and common PostGIS
functions. It is a community dependency, so the first technical spike must
prove all of the following against the pinned Diesel and `diesel-async`
versions:

1. Encode and decode `Point` and `MultiPolygon` with the expected SRID.
2. Run `ST_DWithin`, `ST_Covers`, and boundary queries asynchronously.
3. Confirm GiST indexes are used with `EXPLAIN (ANALYZE, BUFFERS)`.
4. Generate a stable Diesel schema without duplicate custom spatial types.
5. Round-trip the types in integration tests against the production PostgreSQL
   and PostGIS major versions.

If the extension fails the spike, retain Diesel and isolate a small number of
parameterized `diesel::sql_query` spatial statements in the geography adapter.
Changing the accepted persistence library is not the fallback.

## API style

The API is contract-first in behavior even though Utoipa generates the OpenAPI
document from Rust types and handlers:

- Stable resources and commands under `/api/v1`
- RFC 9457-style problem details with project-specific error codes
- Cursor pagination for public feeds
- Idempotency keys for retried commands
- Explicit request/response DTOs with input size and range validation
- UTC instants in RFC 3339; jurisdiction time zones are presentation and policy
  inputs
- OpenAPI diff checks in CI and a generated TypeScript client for the web app

GraphQL, gRPC, and WebSockets are deferred. They can be added for a demonstrated
client or streaming requirement without replacing the public REST contract.

## Identity and authorization

PowerTo delegates authentication to an OIDC provider and validates issuer,
audience, signature, expiry, and authorized flow. The API owns authorization
and civic eligibility; an identity-provider role alone cannot approve a
moderation decision or make a person geographically eligible to vote.

Keycloak is suitable for local development and as a deployable reference
because it can broker OIDC and SAML identity providers. Production may use a
managed or government provider behind the same OIDC contract. The choice of
production operator remains open.

Authentication data and eligibility data are deliberately distinct:

- OIDC subject: who authenticated
- Local platform profile: what the account may do
- Verification claim: what evidence was checked, by whom, and until when
- Eligibility snapshot: why this subject could participate in this poll under
  this rule version

## Web application

The web application is not the backend source of truth. Next.js provides public
server-rendered pages, progressive enhancement, authenticated navigation, and a
small backend-for-frontend surface only for secure browser session handling.
All civic commands still go through the Rust API.

The web release gate includes WCAG 2.2 AA automated checks plus manual keyboard
and screen-reader scenarios. React Aria provides behavior; PowerTo owns its
visual system. The Android and iOS clients consume the same OpenAPI but remain
independently deployable products.

## Mobile clients and sensor evidence

Android and iOS are accepted target clients. The proposed implementation shares
mobile domain, application, API, offline outbox, and upload orchestration through
Kotlin Multiplatform. Android keeps a Jetpack Compose UI and native Kotlin
adapters; iOS keeps a SwiftUI UI and native Swift adapters.

Platform adapters own camera, current location, geofence monitoring, motion
recording, background execution, secure credentials, and persistent upload.
Ktor, Kotlin serialization/coroutines, and Room KMP are proposed for API access,
offline state, and the local command outbox. Atlas applies only to the backend
PostgreSQL database; Room owns versioned local SQLite migrations and checked-in
schemas.

Road surveys are explicit, visibly active, safety-gated capture sessions. Raw
motion/location batches are encrypted locally, uploaded resumably to the
configured object provider, and processed asynchronously. Public segment maps
expose aggregates, method versions, sample sufficiency, and uncertainty—not
individual trips or a definitive claim from one device. See
[mobile capture and road sensing](mobile-sensing.md), [ADR 0013](decisions/0013-mobile-capture-and-road-sensing.md),
and [ADR 0014](decisions/0014-kotlin-multiplatform-native-mobile.md).

MapLibre is the proposed renderer across web and native mobile clients. Styles,
glyphs, sprites, and vector-tile endpoints remain configuration so a deployment
can use a managed OSM-derived provider or self-host. The public
`tile.openstreetmap.org` service is not a production/offline tile backend: its
policy forbids bulk/offline download and provides no SLA. Every map retains the
required OpenStreetMap and data-provider attribution.

## Background jobs without a broker

Commands insert an outbox record in the same transaction as domain state. The
worker claims rows with PostgreSQL locking, records attempts, uses exponential
backoff, and sends poison messages to an inspectable failed state. Handlers are
idempotent.

Add Redis or a message broker only when measured throughput, scheduling, fanout,
or independent-service ownership exceeds what the outbox can safely provide.

## User media storage

Photos and videos use a media port; private raw road-survey batches use a
separate sensor-batch port. Both will be implemented by the selected Cloudflare
R2, AWS S3, or Google Cloud Storage infrastructure adapters. The foundation
currently contains only validated, non-secret provider selection—not SDK
adapters or upload grants. One provider is the default for new writes per
environment; each record retains its provider-neutral locator so old objects
remain accessible during a provider change.

The API issues short-lived direct-upload instructions to a private quarantine
namespace. A worker verifies content and size, scans it, removes metadata, and
creates sanitized image/video derivatives. Only ready derivatives can be
delivered. URLs, citizen filenames, and provider SDK types are not persisted in
the domain. See the [media storage design](media-storage.md) and
[ADR 0011](decisions/0011-portable-media-storage.md).

## Observability

Instrument API and worker code with structured `tracing` spans and the
OpenTelemetry Rust SDK. Both processes export OTLP to the OpenTelemetry
Collector, which routes metrics to VictoriaMetrics, logs to VictoriaLogs, and
traces to VictoriaTraces. Application code never exports directly to a Victoria
backend.

The first operational view needs:

- request count, latency, and error rate by low-cardinality route
- database pool saturation and query latency
- outbox age, retries, and failures
- authentication and authorization failures without personal data
- issue, moderation, and vote command outcomes as privacy-safe metrics

Start with single-node Victoria components. `vmalert` evaluates rules and sends
notifications through an Alertmanager-compatible notifier. Grafana is an
optional dashboard UI, not a telemetry store. Never place tokens, addresses,
ballot identifiers, evidence URLs, media keys, or request bodies in logs,
metrics, or trace attributes. See [ADR 0010](decisions/0010-opentelemetry-victoria-observability.md).

## Intentionally not selected for the MVP

| Technology | Reason for deferral |
| --- | --- |
| Hyperledger Fabric | No established multi-institution validator governance, substantial operational cost, and no official Rust application/chaincode API |
| Hyperledger Iroha 2 | The preferred open-source Rust candidate if a consortium ledger becomes mandatory, but still unnecessary before validators, governance, and a threat model exist |
| Microservices | Domain and team boundaries are not yet stable; distributed transactions would reduce correctness |
| Kubernetes and service mesh | More operational surface than two application processes require |
| Kafka or Redis queues | PostgreSQL outbox covers the initial reliability and volume needs |
| Elasticsearch/OpenSearch | PostgreSQL text and spatial search should be measured first |
| Custom password authentication | Unnecessary security liability; use OIDC |
| Storing uploads in PostgreSQL | Bloats backups and mixes object access with transactional data |
| Direct provider SDKs in use cases | Couples civic behavior and persisted data to one cloud |
| Diesel migrations or startup migrations | Atlas is the accepted sole migration owner; replica startup must be race-free |
| Treating raw phone vibration as a certified road index | Device, mount, vehicle, speed, route, and method bias require calibration and confidence-scored aggregation |

## First technical spikes

Before building product screens, validate the riskiest integrations with small,
discardable tests:

1. Axum + OIDC JWT verification and jurisdiction-scoped authorization.
2. Diesel + `diesel-async` transaction with a uniqueness race between two vote
   attempts.
3. Atlas replay plus Diesel schema generation for multi-schema PostGIS types,
   indexes, and constraints using only the selected Atlas edition.
4. Diesel + PostGIS types and index-aware proximity/boundary queries.
5. Atomic write of domain record, audit event, and outbox row.
6. Utoipa OpenAPI generation and TypeScript client compatibility.
7. Direct quarantine upload, processing, and delivery against R2, S3, and GCS.
8. KMP offline workflow plus native camera/location/motion/background adapters
   on representative Android and iOS devices.
9. A 30–60 minute road-survey experiment across devices, mounts, vehicles,
   speeds, and ground-truth segments, including interrupted resumable upload.
10. End-to-end OTLP routing and redaction with all three Victoria backends,
    proving that mobile routes and motion samples never enter telemetry.

Passing these spikes is the exit criterion for the proposed integrations and
for declaring the accepted provider implementations production-ready.
