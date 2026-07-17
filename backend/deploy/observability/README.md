# Local observability stack

This development-only stack receives OTLP from PowerTo, processes it in the
OpenTelemetry Collector, and writes each signal to its Victoria backend:

```text
PowerTo API / worker -> OTLP -> Collector -> VictoriaMetrics (metrics)
                                        +-> VictoriaLogs    (logs)
                                        +-> VictoriaTraces  (traces)
```

The applications never export directly to a Victoria component. This keeps the
instrumentation vendor-neutral and gives one place to batch, retry, normalize,
and remove known unsafe attributes.

## Start and verify

Run from `backend`:

```sh
docker compose -f deploy/observability/compose.yaml up -d
docker compose -f deploy/observability/compose.yaml \
  --profile check run --rm stack-ready
```

Configure a local PowerTo process to use the Collector's OTLP/gRPC receiver:

```sh
export OTEL_EXPORTER_OTLP_ENDPOINT=http://127.0.0.1:4317
export POWERTO_ENVIRONMENT=local
```

Open the local interfaces:

| Signal | Interface |
| --- | --- |
| Metrics | <http://127.0.0.1:8428/vmui> |
| Logs | <http://127.0.0.1:9428/select/vmui> |
| Traces | <http://127.0.0.1:10428/select/vmui> |
| Collector health | <http://127.0.0.1:13133/> |

Stop containers without deleting their named volumes:

```sh
docker compose -f deploy/observability/compose.yaml down
```

## Privacy and production boundary

This stack binds every published port to localhost, uses unencrypted traffic
inside its Docker network, keeps seven days of local data, and uses in-memory
Collector queues. It is not a production deployment.

Production requires private networking, TLS and authentication, explicit
tenant/retention/capacity decisions, persistent queue evaluation, backups,
alerts, and image digests. VictoriaTraces is still a `0.x` project and needs a
production-readiness review.

The privacy processors delete a small set of known-risk attributes as defense
in depth. They cannot make arbitrary application text safe. Instrumentation
must never emit citizen-supplied content, identity/contact data, exact
locations or routes, authorization tokens, signed object URLs, media, or raw
accelerometer samples. Citizen, issue, media, route, and coordinate identifiers
must not become metric labels because they are sensitive and unbounded.

The `deltatocumulative` processor is currently alpha and stores conversion
state in memory. The Rust SDK is configured for cumulative metrics already;
before scaling the Collector horizontally, decide whether to remove the
processor or provide consistent routing for any delta-producing clients.

## Version update policy

Images are pinned to release tags rather than mutable `latest` tags. Update one
component at a time, read its release notes, validate the Collector config, run
the external readiness check, and prove ingestion for all three signals before
merging. Production manifests should pin the tested tags by digest.

Official protocol references:

- [VictoriaMetrics OTLP ingestion](https://docs.victoriametrics.com/victoriametrics/data-ingestion/opentelemetry-collector/)
- [VictoriaLogs OTLP ingestion](https://docs.victoriametrics.com/victorialogs/data-ingestion/opentelemetry/)
- [VictoriaTraces OTLP ingestion](https://docs.victoriametrics.com/victoriatraces/data-ingestion/opentelemetry/)
