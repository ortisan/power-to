# PowerTo

**Open-source civic infrastructure for turning local problems into transparent,
accountable action.**

PowerTo helps residents report problems that affect their communities, capture
on-site evidence with Android and iOS devices, build local support, and follow
what happens next. The long-term product connects citizens, moderators, public
institutions, and service providers through an auditable process—from the first
report to delivery and public evaluation.

> [!IMPORTANT]
> **Project status: first backend vertical slice.** The repository now contains
> a compiling Rust API/worker workspace and an owner-scoped issue-submission
> flow backed by PostgreSQL/PostGIS. A submission atomically persists its civic
> record, restricted submitter context, hash-linked audit event, idempotency
> record, and transactional outbox message. Protected routes now validate OIDC
> JWT access tokens and map verified identities to private local accounts.
> Outbox processing, moderation, voting, a packaged reference identity provider,
> real R2/S3/GCS adapters, and the web, Android, and iOS clients have not been
> built. PowerTo is not suitable for real civic voting or public-sector
> procurement.

## Why PowerTo

Local problems often disappear between fragmented reporting channels, unclear
priorities, and processes that residents cannot inspect. PowerTo is designed to
make that path visible:

- affected communities participate first;
- rules and prioritization are versioned and explainable;
- personal data is separated from public civic records;
- decisions and state changes produce verifiable audit evidence;
- people can track whether an accepted problem was actually resolved.

The platform is intended for civic prioritization and accountability. It does
not claim to replace legally binding elections, procurement law, emergency
services, or government authority.

## Product flow

The long-term workflow is:

```text
report -> moderation -> local vote -> prioritization -> feasibility
       -> proposals -> approval -> delivery -> citizen evaluation -> resolution
```

The recommended MVP is deliberately smaller:

1. Sign in through an external identity provider.
2. Submit a geographically scoped issue from web, Android, or iOS, with mobile
   photo/video capture and a confirmed affected point or area.
3. Moderate, request clarification, publish, reject, or appeal the issue.
4. Verify territorial eligibility and allow one effective vote per poll.
5. Publish privacy-safe totals, rule versions, status history, and audit
   receipts.
6. Notify participants about relevant state changes.
7. Run a feature-flagged, opt-in road-sensing pilot without using its output to
   decide votes or automatically declare a road defective.

Budgeting, provider proposals, automatic selection, government approval,
delivery management, payments, and blockchain validators remain outside the
MVP until the first jurisdiction, legal authority, and operating governance are
validated. See [MVP scope](docs/product/mvp-scope.md) for the complete boundary.

## Architecture

The target backend is a modular monolith organized with Clean Architecture. It
starts with one Rust API, one Rust worker, and one transactional source of
truth; network services are introduced only for demonstrated boundaries.

```text
Web / PWA + Android / iOS
          |
       Rust API -------- OIDC identity provider
          |
          +---- PostgreSQL + PostGIS
          +---- transactional outbox ---- Rust worker
          +---- media port --------------- R2 / S3 / GCS
          |
       OTLP telemetry
          |
 OpenTelemetry Collector
      /        |         \
 metrics     logs       traces
    |          |           |
Victoria-  Victoria-   Victoria-
Metrics     Logs        Traces
```

Dependencies point inward:

```text
domain <- application <- adapters <- API / worker composition roots
```

Domain rules do not import Axum, Diesel, OpenTelemetry, or cloud SDKs. The
[architecture overview](docs/architecture/overview.md) describes boundaries,
data ownership, voting transactions, and deployment evolution.

## Technology decisions

Accepted decisions are commitments made by the project owner. Proposed choices
must pass the documented technical spikes before implementation is treated as
production-ready.

| Area | Decision | Status |
| --- | --- | --- |
| Architecture | Clean Architecture boundaries | Accepted |
| Backend | Rust stable, edition 2024 | Accepted |
| Persistence | Diesel with `diesel-async` for Tokio | Accepted / first adapter implemented |
| Database migrations | Atlas versioned SQL migrations; no startup migrations | Accepted |
| Observability | OpenTelemetry/OTLP Collector with VictoriaMetrics, VictoriaLogs, and VictoriaTraces | Accepted |
| User media | Provider-neutral port for Cloudflare R2, AWS S3, and Google Cloud Storage | Accepted |
| Mobile clients | Android/iOS capture, offline work, bounded geofencing, and explicit road surveys | Accepted |
| Mobile implementation | KMP shared core with Jetpack Compose and SwiftUI | Proposed |
| Application shape | Modular monolith with separate API and worker processes | Proposed / foundation implemented |
| Database | PostgreSQL with PostGIS and a transactional outbox | Proposed / issue intake implemented |
| HTTP API | Axum/Tokio, REST/JSON, OpenAPI with Utoipa | Proposed / first routes implemented |
| Web | Next.js, React, and TypeScript with an accessibility-first PWA | Proposed |
| Identity | OpenID Connect; Keycloak as the local/reference provider | Proposed / resource-server adapter implemented |

Exact versions, alternatives, and spike criteria live in the
[technology stack](docs/architecture/technology-stack.md).

### Database ownership

Atlas is the only migration engine. Reviewed SQL migrations and `atlas.sum`
form the deployment history; a dedicated release job validates and applies
them. Diesel and `diesel-async` own parameterized Rust access after Atlas has
migrated the database. API and worker replicas never migrate the database at
startup.

### First implemented use case

The API currently exposes an authenticated-account-shaped boundary for two
private operations:

- `POST /api/v1/me/issues` submits a confirmed civic problem using a required
  UUID `Idempotency-Key`;
- `GET /api/v1/me/issues/{issue_ref}` retrieves that issue only when it belongs
  to the same account.

The external issue reference is opaque and owner filtering happens in the
persistence query. Exact retries return the same issue's current owner-scoped
representation without creating another record; reuse of a key for a different
command is rejected. Responses are private and non-cacheable, and
request bodies, actor identifiers, idempotency keys, free-form text, and exact
coordinates are excluded from telemetry.

These routes accept an OIDC JWT access token through `Authorization: Bearer`.
The Rust adapter discovers the provider, validates the exact issuer, API
audience, `at+jwt` token type, RS256 signature, lifetime, and signing key, then
atomically resolves or provisions a private local account from `(issuer,
subject)`. Tokens, names, and email addresses are not persisted. Suspended or
closed accounts cannot act, and provider/JWKS failures fail closed.

For local development only, an explicitly enabled insecure account header can
replace OIDC while the API is both in the `local` environment and bound to a
loopback address. The two modes cannot be enabled together. Categories are
currently validated slugs rather than references to a canonical jurisdiction
catalog, and no jurisdiction is derived yet.

### Photos and videos

Planned deployments choose Cloudflare R2, AWS S3, or Google Cloud Storage for
new media. The target database model records the provider and an opaque
immutable locator, so existing objects remain readable during a provider
change.

```text
authorized direct upload -> private quarantine -> inspect and scan
                         -> strip metadata / transcode -> safe derivative
                         -> short-lived authorized delivery
```

Under this design, original uploads are never public, and signed URLs, citizen
filenames, and provider URLs are not persisted. Read the [media storage
design](docs/architecture/media-storage.md) for the security flow and provider
contract.

### Mobile location and road sensing

Issue geography and device geofencing are separate. Citizens confirm an issue's
point or affected area in the app. Operating-system geofences provide delayed,
capacity-limited reminders for a small set of followed issues; they do not prove
presence, residence, or voting eligibility.

During an explicit, visibly active road survey, the app records synchronized
motion and location batches for asynchronous processing:

```text
native capture -> encrypted offline batch -> resumable upload
               -> normalize and map-match -> segment observation + confidence
               -> privacy-thresholded map of good and poor quality indications
```

Vehicle, suspension, speed, phone mount, orientation, and device hardware all
affect vibration. One trip is supporting evidence, not a certified road-quality
measurement. Public results require calibrated, versioned methods, multiple
eligible observations, uncertainty, and aggregation that cannot reveal a
person's route. See [mobile capture and road
sensing](docs/architecture/mobile-sensing.md).

### Voting integrity and blockchain

Blockchain is not part of the MVP. The current proposal uses PostgreSQL
transactions, append-only hash-linked audit events, signed checkpoints, opaque
receipts, and privacy-safe public aggregates.

The original Hyperledger Fabric proposal is on hold: it lacks established
validator governance and conflicts with an all-Rust backend. If independent
institutions later commit to operating a permissioned network, Hyperledger
Iroha 2 is the first open-source Rust candidate to evaluate against simpler
transparency-log designs and an explicit threat model.

## Engineering principles

- **Local first:** participation rules reflect the affected territory without
  exposing a citizen's precise address.
- **Privacy by design:** identity, location, ballots, and public projections
  have separate data boundaries; free-form citizen content never enters
  telemetry.
- **Explainable decisions:** eligibility, weights, thresholds, moderation, and
  rankings are deterministic, versioned, and reviewable.
- **Accessible participation:** public flows target WCAG 2.2 AA, keyboard and
  screen-reader use, responsive layouts, and low-bandwidth operation.
- **Operational restraint:** no microservices, broker, Kubernetes, or
  blockchain without evidence that its cost solves a real constraint.
- **Provider portability:** identity, media, notifications, and observability
  stay behind standards or explicit application ports.
- **Evidence with uncertainty:** sensor-derived maps publish confidence,
  methodology, sufficiency, and limitations instead of overstating precision.

## Documentation map

| Document | Purpose |
| --- | --- |
| [MVP scope](docs/product/mvp-scope.md) | Long-term product, first release, actors, lifecycle, and open policy questions |
| [Architecture overview](docs/architecture/overview.md) | Clean Architecture layers, domain boundaries, data, voting, and deployment |
| [Technology stack](docs/architecture/technology-stack.md) | Accepted/proposed choices, versions, alternatives, and technical spikes |
| [User media storage](docs/architecture/media-storage.md) | R2/S3/GCS port, upload states, processing, privacy, and migration |
| [Mobile capture and road sensing](docs/architecture/mobile-sensing.md) | Android/iOS boundaries, geofences, sensor pipeline, calibration, and safety |
| [Backend guide](backend/README.md) | Rust workspace, local dependencies, runtime configuration, and quality gates |
| [Issue proposal model](docs/models/issue-proposal-model.md) | Current structure for citizen issue submissions |
| [Architecture decisions](docs/architecture/decisions/0000-adr-template.md) | Decision history and the ADR template |

Important decision records include:

- [Rust backend](docs/architecture/decisions/0004-rust-backend.md)
- [Diesel persistence](docs/architecture/decisions/0005-diesel-persistence.md)
- [Relational vote ledger before blockchain](docs/architecture/decisions/0008-relational-vote-ledger.md)
- [Clean Architecture](docs/architecture/decisions/0009-clean-architecture.md)
- [OpenTelemetry and Victoria backends](docs/architecture/decisions/0010-opentelemetry-victoria-observability.md)
- [Portable media storage](docs/architecture/decisions/0011-portable-media-storage.md)
- [Atlas migrations](docs/architecture/decisions/0012-atlas-database-migrations.md)
- [Mobile capture and road-sensing evidence](docs/architecture/decisions/0013-mobile-capture-and-road-sensing.md)
- [Proposed Kotlin Multiplatform clients](docs/architecture/decisions/0014-kotlin-multiplatform-native-mobile.md)

## Run the backend locally

The backend exposes liveness, PostgreSQL readiness, its OpenAPI document, and
the first private issue-intake routes. With Rust, Docker Compose, and Atlas
installed:

```bash
git clone https://github.com/ortisan/power-to.git
cd power-to/backend
docker compose up -d --wait postgres
export DATABASE_URL='postgres://powerto:powerto-local-only@127.0.0.1:5432/powerto'
(cd db && atlas migrate apply --env local)
export POWERTO_DATABASE_URL="$DATABASE_URL"
export POWERTO_ALLOW_INSECURE_LOCAL_ACTOR_HEADER=true
cargo run -p powerto-api
```

The insecure header is shown only to make the local issue routes testable; it is
disabled by default and the process refuses it outside `local` or on a
non-loopback bind. The worker starts but does not consume outbox messages yet.
Provider credentials are not needed because media adapters are not implemented.
Mobile photo/video capture, geofences, and accelerometer-based road surveys
also remain planned. Complete configuration and a synthetic request example
are in the [backend guide](backend/README.md).

## Run the documentation locally

With Node.js and npm installed:

```bash
cd power-to/docs/website
npm ci
npm start
```

Create a production documentation build with:

```bash
npm run build
```

## Contributing

PowerTo welcomes product, policy, accessibility, security, documentation, and
engineering contributions. Before opening a pull request:

1. Read the [contribution guide](CONTRIBUTING.md) and
   [code of conduct](CODE_OF_CONDUCT.md).
2. Check whether the change affects an accepted ADR or needs a new decision.
3. Keep commits compatible with
   [Conventional Commits](https://www.conventionalcommits.org/).
4. Include tests or verification appropriate to the change and avoid personal
   or real citizen data in fixtures.

## License

PowerTo is released under the [MIT License](LICENSE).
