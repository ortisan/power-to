# PowerTo backend

This directory contains the executable backend foundation and the first civic
vertical slice: an OIDC-authenticated account can submit a confirmed issue and
read that same issue back within its private owner scope. Moderation, voting,
media upload, jurisdiction policy, and road-survey use cases are not implemented
yet.

## Structure

```text
apps/api          Axum composition root, OIDC, issue/health routes, and OpenAPI
apps/worker       background-process composition root; no outbox consumer yet
crates/domain     framework-independent issue types and invariants
crates/application issue use cases and purpose-specific ports
crates/adapters   OIDC/JWKS, diesel-async/PostgreSQL, storage, and telemetry
crates/test-support deterministic test doubles, including issue persistence
db                Atlas versioned PostgreSQL/PostGIS migrations
deploy/observability OpenTelemetry Collector and Victoria development stack
```

The compile-time dependency direction is:

```text
domain <- application <- adapters <- API / worker
```

Atlas—not Diesel and not either executable—is the only migration engine.

## Prerequisites

- the Rust toolchain pinned in `rust-toolchain.toml`;
- Docker with Compose for PostgreSQL/PostGIS and local telemetry; and
- an Atlas CLI distribution compatible with the workflow in [`db/README.md`](db/README.md).

The current `postgis/postgis:18-3.6` development image publishes an amd64
runtime. Docker Desktop on Apple Silicon can run it through amd64 emulation;
expect an architecture warning and slower startup. This local image choice does
not prescribe the production database provider.

## Run locally

Start PostgreSQL/PostGIS from this directory:

```sh
cd backend
docker compose up -d --wait postgres
export DATABASE_URL='postgres://powerto:powerto-local-only@127.0.0.1:5432/powerto'
```

The Docker image initializes its PostGIS objects in an auxiliary database. A
checked-in init script creates `powerto` from `template0`, leaving it clean for
Atlas to own the schema history; do not point Atlas at the auxiliary database.

Apply the immutable Atlas migration history, then start the optional telemetry
stack:

```sh
(cd db && atlas migrate apply --env local)
docker compose -f deploy/observability/compose.yaml up -d
```

Run the API with the same migrated database:

```sh
export POWERTO_DATABASE_URL="$DATABASE_URL"
export POWERTO_ENVIRONMENT=local
export POWERTO_PRIVACY_NOTICE_VERSION=privacy-v1
export OTEL_EXPORTER_OTLP_ENDPOINT=http://127.0.0.1:4317
cargo run -p powerto-api
```

In another shell:

```sh
curl -i http://127.0.0.1:8080/health/live
curl -i http://127.0.0.1:8080/health/ready
curl http://127.0.0.1:8080/api/openapi.json
```

## Implemented issue intake

The first vertical slice uses Rust, Axum, Diesel with `diesel-async`, and a
PostgreSQL/PostGIS transaction. It exposes:

| Method and path | Current behavior |
| --- | --- |
| `POST /api/v1/me/issues` | Validates and submits one confirmed point issue |
| `GET /api/v1/me/issues/{issue_ref}` | Returns the issue only inside the requesting account's scope |

`POST` requires a UUID in `Idempotency-Key`. The first successful call returns
`201`; an exact retry returns the same issue's current owner-scoped
representation with `200` and `Idempotency-Replayed: true`, without a second
write. Reusing the same key for a changed command returns `409`. The raw key is
never stored: persistence keeps its SHA-256 digest and a versioned normalized
command fingerprint. Fingerprinting and replay lookup occur before mutable
submission policies are evaluated, so a historical exact retry is not rejected
merely because a newer policy has been deployed. The external issue reference
is a random UUIDv4; the internal time-ordered identifier is not exposed.

Each successful new submission commits the issue, restricted account/location
context, idempotency record, append-only hash-linked audit event, and a
privacy-minimal outbox message in one database transaction. The worker does not
claim or process those outbox messages yet.

All issue responses use `Cache-Control: private, no-store`. Owner and opaque
reference are filtered in the same database query, and a missing issue and an
issue owned by another account produce the same `404`. Telemetry records bounded
HTTP method, route template, and status attributes; it does not record the
request body, issue/account references, idempotency key, free-form citizen text,
or coordinates.

## Authentication

Protected routes require an OIDC JWT access token. At startup, the API uses
OpenID Connect Discovery, requires the returned issuer to exactly match
`POWERTO_OIDC_ISSUER`, retrieves a bounded JWKS document, and caches its keys.
Access tokens must use `typ: at+jwt` (or `application/at+jwt`), an RS256 signing
key from that JWKS, the configured audience, and valid `iss`, `sub`, `exp`, and
`iat` claims. `nbf` is enforced when present. Unknown signing keys trigger a
rate-limited refresh; a known cached key may remain usable during a temporary
refresh failure.

After cryptographic validation, `(issuer, subject)` is atomically mapped to a
UUIDv7 local account in `private.account_identities`. The database stores no
token, name, or email. An `active` account may act; `suspended` and `closed`
accounts receive `403`. Missing or invalid credentials receive `401` with
`WWW-Authenticate: Bearer`, and provider/directory outages receive `503`.

Keycloak is the reference provider but is not packaged in Compose yet. Its API
client must include this API in the token audience and enable **Use at+jwt as
access token header type**. Mobile and web public clients must use Authorization
Code with PKCE; PowerTo does not collect passwords or use the resource-owner
password flow. See the [Keycloak `at+jwt` setting](https://www.keycloak.org/2025/04/keycloak-2620-released),
[RFC 9068](https://www.rfc-editor.org/rfc/rfc9068), and [OpenID Connect
Discovery](https://openid.net/specs/openid-connect-discovery-1_0.html).

Configure a provider and run the API:

```sh
export POWERTO_OIDC_ISSUER='https://identity.example/realms/powerto'
export POWERTO_OIDC_AUDIENCE='powerto-api'
cargo run -p powerto-api
```

Outside `local`, both OIDC variables are mandatory. Plain HTTP discovery/JWKS
is rejected, except for a loopback provider in the `local` environment.

### Insecure local test actor

A deliberately insecure development adapter can replace OIDC only when
`POWERTO_ENVIRONMENT=local` and `POWERTO_HTTP_ADDRESS` is a loopback address:

```sh
export POWERTO_ALLOW_INSECURE_LOCAL_ACTOR_HEADER=true
cargo run -p powerto-api
```

The process refuses this setting outside `local`, on a non-loopback bind, or
when OIDC variables are also set. The
`x-powerto-local-account-id` header is only a local test seam; it is not an
authentication design and must never be exposed over a network.

With that local-only mode enabled, this synthetic request exercises the route:

```sh
curl -i http://127.0.0.1:8080/api/v1/me/issues \
  -H 'Content-Type: application/json' \
  -H 'Idempotency-Key: aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa' \
  -H 'x-powerto-local-account-id: 11111111-1111-4111-8111-111111111111' \
  --data-binary '{
    "title": "Blocked drain in a synthetic test location",
    "category_id": "drainage",
    "summary": "Synthetic data used only to exercise local issue intake.",
    "problem_statement": "The test fixture represents a drain that is blocked after rain.",
    "affected_community": "People using the fictional public square.",
    "desired_outcome": "Inspect and clear the fictional drain.",
    "location": {"longitude": 0.0, "latitude": 0.0},
    "geometry_source": "map_selection",
    "location_confirmed": true,
    "location_label": "Synthetic public square",
    "public_attribution": false,
    "privacy_notice_version": "privacy-v1",
    "privacy_notice_accepted": true
  }'
```

Use the returned `reference` with `GET /api/v1/me/issues/{issue_ref}` and the
same local account header. The current `category_id` is only a validated slug;
it is not checked against a canonical category catalog, and jurisdiction is not
derived or stored by this use case. The confirmed point is returned only by the
owner-scoped API today. A public projection and its location-generalization
policy must exist before public issue reads are added.

The worker uses the same database and OTLP variables:

```sh
cargo run -p powerto-worker
```

If `OTEL_EXPORTER_OTLP_ENDPOINT` is absent, both executables retain structured
JSON stdout logs and do not construct OTLP exporters. The Collector is not a
readiness dependency; PostgreSQL is.

## Runtime configuration

| Variable | Required | Default | Purpose |
| --- | --- | --- | --- |
| `POWERTO_DATABASE_URL` | yes | none | PostgreSQL URL; never logged |
| `POWERTO_DATABASE_POOL_SIZE` | no | `10` | Positive maximum pool size |
| `POWERTO_DATABASE_TIMEOUT_MS` | no | `500` | Pool connection and readiness timeout |
| `POWERTO_HTTP_ADDRESS` | API only, no | `127.0.0.1:8080` | API listen socket |
| `POWERTO_ENVIRONMENT` | no | `local` | OTLP deployment environment resource |
| `POWERTO_PRIVACY_NOTICE_VERSION` | outside `local` | `privacy-v1` in `local` | Privacy notice accepted for new issue submissions |
| `POWERTO_OIDC_ISSUER` | outside `local`; paired with audience | none | Exact OIDC issuer used for discovery and token validation |
| `POWERTO_OIDC_AUDIENCE` | outside `local`; paired with issuer | none | Audience required in API access tokens |
| `POWERTO_OIDC_CLOCK_SKEW_SECONDS` | no | `30` | Allowed clock skew for token claims, from 0 through 300 |
| `POWERTO_OIDC_HTTP_TIMEOUT_SECONDS` | no | `3` | Discovery and JWKS timeout, from 1 through 30 |
| `POWERTO_OIDC_JWKS_REFRESH_SECONDS` | no | `300` | Key-cache refresh interval, from 30 through 86400 |
| `POWERTO_ALLOW_INSECURE_LOCAL_ACTOR_HEADER` | no | `false` | Enables the local loopback-only test actor; never production authentication |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | no | none | Enables OTLP/gRPC export to the Collector |
| `RUST_LOG` | no | `info` | `tracing-subscriber` filter |

Provider credentials do not belong in this repository. Future R2, S3, and GCS
adapters will use workload identity or each SDK's standard credential chain.
The current code validates provider-neutral non-secret configuration but does
not yet issue upload URLs. Android/iOS apps, photo/video capture, geofence
reminders, accelerometer road surveys, and their offline synchronization remain
planned rather than implemented.

## Quality checks

GitHub Actions runs these checks for every pull request and `main` push. The
workflow uses the repository's pinned Rust toolchain, Node.js 24, Atlas v1.2.0,
and PostgreSQL 18/PostGIS 3.6. Third-party actions are pinned to commit SHAs and
receive only read access to repository contents.

```sh
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace --all-targets
(cd db && atlas migrate validate --dir file://migrations)
docker compose config --quiet
docker compose -f deploy/observability/compose.yaml config --quiet
```

Integration replay of Atlas migrations requires a disposable PostgreSQL 18
database with PostGIS 3.6; see the database README for the safe command. CI
creates a clean database from `template0`, applies every Atlas migration, and
runs all ignored `powerto-adapters` integration tests serially.
