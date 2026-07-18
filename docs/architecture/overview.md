---
id: overview
title: Architecture Overview
sidebar_label: Overview
---

# Architecture Overview

## Architectural direction

PowerTo starts as a **modular monolith organized with Clean Architecture**, a
Rust backend, and separately deployable web, Android, and iOS clients. The
system has one transactional source of truth, clear domain boundaries, and an
outbox for reliable asynchronous work.

This is a deployment choice, not permission to mix every concern. Dependencies
point inward: domain code has no framework dependencies; application use cases
depend on the domain and define ports; outer adapters implement those ports.
HTTP, Diesel, OpenTelemetry, mobile platform APIs, an identity vendor, media
storage, and a notification provider never become inner-layer dependencies.

The architecture optimizes for the following qualities, in order:

1. Trust, auditability, and explainable civic decisions
2. Privacy and least-privilege access to personal and location data
3. Correctness under concurrent voting and workflow transitions
4. Accessibility, low-bandwidth use, and progressive enhancement
5. A small operational footprint that a small team can safely run
6. Evolution into independently deployable services only when evidence demands it

## Current implementation boundary

The repository has moved beyond health-check scaffolding into one complete
backend slice. Rust domain and application code, an Axum inbound adapter, a
Diesel/`diesel-async` PostgreSQL adapter, and Atlas-managed PostGIS tables now
support:

- `POST /api/v1/me/issues` for idempotent submission of a confirmed point
  issue; and
- `GET /api/v1/me/issues/{issue_ref}` for an owner-scoped private read using an
  opaque random reference.

A new submission commits civic state, restricted submitter context, the
idempotency reservation, a hash-linked append-only audit event, and one outbox
message in the same transaction. The worker is only a composition-root
foundation and does not process the outbox yet.

OIDC actor resolution is implemented for both issue routes. The API validates
strict JWT access tokens against provider discovery and cached JWKS, then maps
the verified issuer/subject pair to a private local account. An insecure account
header remains solely as an explicitly enabled local test seam; process
configuration restricts it to the `local` environment and a loopback HTTP bind,
and never enables it at the same time as OIDC.

The current category value is a validated slug, not a canonical
jurisdiction-owned category, and the slice does not derive a jurisdiction.
Moderation, voting, public issue reads, media upload/provider adapters, web and
mobile clients, geofence behavior, and accelerometer road sensing remain
planned.

## System context

```text
 Citizens / Visitors       Moderators       Government       Providers
          |                    |                 |                |
          +--------------------+-----------------+----------------+
                                   |
                    Web/PWA + Android + iOS apps
                                   |
                            PowerTo Rust API
                          /        |         \
                  OIDC Identity  PostgreSQL   Media storage
                                  + PostGIS   R2 / S3 / GCS
                                      |
                                PowerTo Worker
                                      |
                         Notifications / public exports
```

Government and provider integrations are future adapters. The core product
must still provide a useful public record when an external institution has no
API or has not formally joined the platform.

## Containers

| Container | Responsibility | Initial deployment |
| --- | --- | --- |
| Web application | Public pages, accessible forms, authenticated workspace, server rendering, and API client | One Node.js container or managed web runtime |
| Android application | Issue/media/location capture, voting, offline sync, bounded geofence reminders, and explicit road surveys | Installed client; proposed Jetpack Compose UI with KMP shared core |
| iOS application | Issue/media/location capture, voting, offline sync, bounded geofence reminders, and explicit road surveys | Installed client; proposed SwiftUI UI with KMP shared core |
| Rust API | Authentication enforcement, commands, queries, policy evaluation, transactions, and OpenAPI | One stateless container, horizontally replicable |
| Rust worker | Outbox processing, notifications, media checks, exports, and scheduled jobs | Same codebase, separate process |
| PostgreSQL + PostGIS | Transactional records, spatial boundaries and queries, audit events, and outbox | Managed service where possible |
| Evidence object storage | User photos/videos plus private raw sensor batches; media quarantine, sanitized derivatives, and short-lived access | Cloudflare R2, AWS S3, or Google Cloud Storage adapter |
| OIDC provider | Login, MFA, session security, and federation with future government identity | External or managed service; Keycloak is the local/reference implementation |
| OpenTelemetry Collector | Receives, enriches, batches, filters, and exports OTLP signals without application vendor coupling | One gateway process initially |
| VictoriaMetrics stack | VictoriaMetrics for metrics, VictoriaLogs for logs, and VictoriaTraces for traces | Single-node components first; managed or self-hosted |

No container calls another through an internal message broker in the first
release. The worker will claim jobs from the transactional outbox after its
consumer is implemented; today those rows remain pending. Redis, Kafka, and
Kubernetes require a measured need and a new decision record.

## Clean Architecture dependency rule

```text
 Frameworks and drivers
 Axum | Diesel | PostgreSQL | OIDC | R2/S3/GCS | OpenTelemetry
 KMP | Compose | SwiftUI | Android/iOS camera, location and motion APIs
                         |
 Interface adapters      |  HTTP presenters, persistence/provider adapters
                         |
 Application             |  use cases and inbound/outbound ports
                         |
 Domain                  |  entities, value objects, policies, events
                         v
                    dependencies point inward
```

- The **domain layer** contains entities, value objects, policies, invariants,
  and domain events. It has no infrastructure, serialization, or telemetry
  imports.
- The **application layer** coordinates a use case, authorization decision, and
  unit-of-work port. It defines repository, identity, clock, storage, and event
  ports without knowing their implementations.
- **Interface adapters** translate boundaries to and from application types.
  The API crate owns inbound HTTP translation; the outer adapters crate owns
  Diesel records, OIDC claims, and provider payloads. Diesel models never
  become API response types.
- **Frameworks and drivers** configure Axum, Diesel, PostgreSQL, external
  providers, and the OpenTelemetry pipeline. The composition roots are the only
  places that know all layers.
- A write commits domain state, audit evidence, and its outbox messages in the
  same PostgreSQL transaction.
- Observability wraps entry points and adapters. Domain events express business
  facts and are not repurposed as logs or traces.

## Current source layout

The repository contains the executable foundation plus the issue-intake module
shown below. Mobile directories remain target additions, not empty placeholders
that already exist:

```text
backend/
  Cargo.toml                 # workspace and shared dependency versions
  rust-toolchain.toml        # pinned stable toolchain
  apps/
    api/                     # Axum issue/health/OpenAPI adapter and composition root
    worker/                  # database-ready root; outbox processing is pending
  crates/
    domain/                  # framework-free issue identity, values and invariants
    application/             # issue commands/queries and purpose-specific ports
    adapters/                # diesel-async persistence, readiness, config, telemetry
    test-support/            # deterministic readiness and issue-store test doubles
  db/
    migrations/             # Atlas bootstrap, issue intake, OIDC identity, and atlas.sum
    atlas.hcl                # environment-driven migration configuration
  deploy/observability/      # Collector and local Victoria Compose stack

mobile/                      # target; not created yet
  shared/                    # KMP domain, application, data and device ports
  androidApp/                # Jetpack Compose UI and Android adapters
  iosApp/                    # SwiftUI and Apple-platform adapters
```

Start with bounded-context modules inside the `domain` and `application` crates,
not one crate per context. This keeps the workspace navigable while boundaries
are still being discovered. Extract a context only when ownership, reuse, or
dependency pressure makes it demonstrably stable. Procurement, delivery, and
evaluation become modules later; empty placeholders add no value.

The compile-time dependency graph is:

```text
domain <- application <- adapters
   ^            ^           ^
   +------------+-----------+--- api / worker composition roots
```

`domain` cannot depend on any other workspace layer. `application` cannot
depend on `adapters`, Axum, Diesel, or OpenTelemetry.

## Domain boundaries

| Context | Owns | Does not own |
| --- | --- | --- |
| Identity and access | Local profile, platform roles, jurisdiction membership, verification status | OIDC passwords or raw provider tokens |
| Geography | Jurisdictions, administrative boundaries, safe public location, spatial matching | Voting weights or moderation decisions |
| Issues | Issue description, evidence references, affected area, lifecycle | Individual ballots or provider contracts |
| Sensor evidence | Road-survey session, raw-batch reference, derived segment observation, method version, confidence, and lineage | Declaring an issue true or a road professionally certified |
| Moderation | Review case, decision, reason, policy version, appeal | Silent edits to issue history |
| Voting | Poll, rule set, eligibility snapshot, ballot, effective vote | Exact residence evidence or public identity |
| Prioritization | Reproducible ranking snapshots and thresholds | Mutable, unexplained scores |
| Audit | Append-only civic events, privileged actions, receipt and checkpoint evidence | General application debug logs |
| Notifications | Delivery preferences and delivery attempts | Source-of-truth workflow state |
| Procurement (later) | Provider verification, proposal, evaluation rule, award recommendation | Government legal approval or payment |
| Delivery (later) | Work milestones, evidence, citizen acceptance, correction cycle | Original voting history |

Contexts exchange IDs, explicit commands, immutable facts, or versioned read
models. They do not join each other's private tables from application code.

## Data and privacy boundaries

PostgreSQL remains one physical database initially. Separate schemas exist
today; least-privilege runtime and migration roles are still a deployment
follow-up:

- `civic`: public and operational issue, poll, aggregate, and workflow data
- `private`: contact data, identity links, eligibility evidence, and precise
  issue locations and raw survey routes; narrower service-role access is the
  target policy
- `evidence`: media/survey metadata, derived road observations, method versions,
  and privacy-gated segment aggregates; raw bytes remain in object storage
- `audit`: append-only events and checkpoints; the current trigger rejects
  updates and deletes, and a future dedicated application role must only append
- `jobs`: transactional outbox and delivery attempts

The current API has no public issue read. Its private owner response returns the
confirmed point, while `civic.issues.public_location` currently holds that exact
civic problem point under the method `exact-civic-problem-point-v1`. This is
not a device route, residence assertion, or eligibility signal. Before public
maps or lists exist, the project must add a reviewed projection and
jurisdiction-specific generalization policy; omitting private fields during
JSON serialization is not sufficient privacy protection.

Database constraints protect invariants that must survive application bugs,
including one effective ballot per poll and voter pseudonym, valid state
references, and idempotency-key uniqueness. Row-level security is defense in
depth for the most sensitive tables, not a substitute for application
authorization.

## Voting transaction

The first implementation uses PostgreSQL, not blockchain, as the voting
transaction coordinator:

1. The authenticated subject is resolved to a verified eligibility claim.
2. The current, immutable voting-rule version maps that claim to an eligibility
   class without copying a precise address into the ballot.
3. A poll-scoped pseudonym is derived so ballots cannot be linked across polls
   through a public identifier.
4. One serializable transaction checks the poll window, inserts the ballot,
   enforces the uniqueness constraint, appends an audit event, and enqueues the
   aggregate update.
5. The API returns an opaque receipt. Public results expose aggregates and the
   rule version, subject to small-group suppression.
6. Signed, externally published checkpoints make a later rewrite of the audit
   chain detectable. A changed vote, if policy allows it, supersedes an earlier
   event rather than erasing history.

This design still requires a threat model, independent review, key management,
and clear institutional governance. It does not claim the properties of a
public election system.

## API and integration rules

- JSON REST endpoints are versioned under `/api/v1` and described by OpenAPI
  3.1 generated from the Rust code.
- The implemented issue routes are `POST /api/v1/me/issues` and
  `GET /api/v1/me/issues/{issue_ref}`. Both require a verified bearer actor;
  `(issuer, subject)` resolves to a private UUIDv7 account without retaining the
  raw token or profile claims. The loopback-only local header is a test seam.
- Issue submission requires a UUID idempotency key. An exact retry returns the
  same issue's current owner-scoped representation without a second write,
  while reuse for a changed command conflicts. Only the key digest and a
  versioned normalized-command fingerprint are persisted. Replay lookup occurs
  before mutable submission policies are evaluated.
- Workflow updates use optimistic concurrency or an expected aggregate version.
- Errors use one stable problem-details shape and never expose internal errors
  or personal data.
- Long-running work returns an accepted operation and is completed by the
  worker. Server-sent events may be added for progress; WebSockets are not a
  default.
- External identity, geocoding, email, and government systems are adapters with
  timeouts, retries, circuit-breaking policy, and auditable outcomes.
- Mobile offline commands and upload completion are idempotent. Raw media and
  sensor batches use backend-issued direct/resumable upload plans rather than
  passing large bodies through the API.

## Media boundary

The application defines a media-storage port; Cloudflare R2, AWS S3, and Google
Cloud Storage are outer adapters. The database stores a provider-neutral media
ID plus the provider, bucket/namespace, immutable object key, verified digest,
size, detected type, and lifecycle state. It never stores a provider URL.

The planned API authorizes an upload and returns a short-lived direct-upload
grant. New objects enter a private quarantine area. The worker verifies the
uploaded object, enforces size and detected-content policy, scans it, strips
image metadata, and produces safe image or video derivatives. Only ready
derivatives can be delivered to users. See the
[media storage design](media-storage.md) for the state machine and provider
contract.

## Mobile evidence and road sensing

The planned Android and iOS applications capture media and a citizen-confirmed
issue point or affected area. Operating-system geofences are only proximity
hints for a bounded set of followed/nearby issues; they are not accurate
evidence of presence, residence, or voting eligibility.

The planned explicit road-survey session synchronizes motion and location
samples, stores an encrypted offline batch, and uploads it through the
media-provider port. The worker validates, normalizes, map-matches, and derives
a versioned, confidence-scored road-segment observation. A public good/poor
quality map uses thresholded aggregates, never an individual's precise route
or a single phone trip. See [mobile capture and road sensing](mobile-sensing.md).

Kotlin Multiplatform with Jetpack Compose and SwiftUI is proposed for the client
implementation. This keeps shared offline/domain/data logic while sensor,
camera, geofence, and background adapters remain native. It is not accepted
until the physical-device spikes in ADR 0014 pass.

## Deployment evolution

Start with two application processes, one database, media storage, and an
identity provider, plus independently released web and mobile clients. Scale
stateless API replicas before splitting services. A
module becomes an independent service only when it has a distinct scaling,
security, ownership, or availability need that cannot be met inside the
monolith. The outbox and module ports provide a migration seam if that day
comes.

## Observability pipeline

The application emits vendor-neutral OpenTelemetry signals through OTLP:

```text
Rust API / Worker
  tracing + OpenTelemetry SDK
             |
          OTLP TLS
             |
    OpenTelemetry Collector
       |          |          |
       v          v          v
 VictoriaMetrics VictoriaLogs VictoriaTraces
    metrics        logs         traces
       \            |            /
        +------ dashboards -------+
                   |
             vmalert / notifier
```

The Collector is the only application-facing telemetry destination. It applies
resource attributes, memory limits, batching, redaction, sampling, and routing.
VictoriaMetrics components are outer providers and can be changed without a
domain or application code change.

Start with single-node VictoriaMetrics, VictoriaLogs, and VictoriaTraces. Move
to cluster mode only after retention, ingestion rate, query load, or availability
objectives require it. Metrics use cumulative temporality, which is the
preferred VictoriaMetrics representation.

Telemetry never includes a person's name, email, OIDC subject, address,
eligibility evidence, ballot/poll pseudonym, access token, evidence URL, request
body, raw route, motion sample, or free-form citizen content. Metric labels use
route templates and other bounded dimensions; record IDs are not labels.
Redaction tests and cardinality budgets are release gates.

## Required architecture follow-ups

Before a public pilot, create and review:

- A data-protection impact assessment and retention schedule
- A voting and identity threat model, including Sybil attacks, coercion,
  administrator compromise, and small-area re-identification
- The first jurisdiction's versioned eligibility and prioritization rules
- Backup, restore, key-rotation, and incident-response exercises
- Measurable availability, latency, accessibility, RTO, and RPO objectives
- A follow-up PostGIS spike covering jurisdiction boundaries, generalized
  public geometry, and representative `ST_DWithin` queries
- Android/iOS physical-device surveys covering background behavior, offline
  recovery, battery/thermal cost, permission denial, and resumable uploads
- A road-sensing research plan with ground truth, device/vehicle bias analysis,
  aggregation privacy, method versioning, and non-misleading public language
