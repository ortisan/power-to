---
id: 0010-opentelemetry-victoria-observability
title: OpenTelemetry with VictoriaMetrics Backends
---

# OpenTelemetry with VictoriaMetrics Backends

## Status

ACCEPTED on 2026-07-16 by explicit project-owner decision.

Supersedes [ADR 0003](0003-opensource-observability-tools.md).

## Context

PowerTo needs correlated metrics, logs, and traces without coupling application
code to a storage vendor. Telemetry may contain civic workflow context, so
cardinality, access, retention, and personal-data controls are part of the
architecture rather than operational afterthoughts.

## Decision

Instrument the Rust API and worker with `tracing` and the OpenTelemetry Rust
SDK. Export OTLP to an OpenTelemetry Collector gateway. The application never
exports directly to a Victoria component.

The Collector routes the three signals to the VictoriaMetrics stack:

- metrics to VictoriaMetrics;
- structured logs to VictoriaLogs;
- traces to VictoriaTraces.

The Collector owns batching, memory limits, redaction, resource enrichment,
sampling, retry, and routing. Metrics use cumulative temporality. `vmalert`
evaluates recording and alerting rules and sends notifications through an
Alertmanager-compatible component. Grafana may be used as a query and dashboard
UI, but it is not a telemetry system of record.

Start with the single-node form of each Victoria component. Cluster mode is a
capacity and availability evolution, not the initial topology. All components
and Rust crates are version-pinned because the OpenTelemetry Rust traces,
metrics, and logs APIs are currently marked beta.

Telemetry delivery is best effort and bounded. Collector or backend failure
must not fail a civic command or create unbounded application memory growth.

## Data policy

Do not emit names, email addresses, OIDC subjects, tokens, precise locations,
eligibility evidence, ballot pseudonyms, media URLs, request bodies, or
free-form citizen content. Use route templates and bounded enums as dimensions;
never use record IDs or object keys as metric labels.

Access to telemetry is role-restricted and audited. Transport uses TLS and
authenticated OTLP endpoints outside local development. Retention is configured
per signal and must follow the project's data-protection schedule.

## Consequences

### Positive

- Instrumentation remains vendor-neutral and the Collector is a replacement
  seam for every backend.
- Metrics, logs, and traces share resource and correlation attributes.
- The selected backend stack is open source and can be self-hosted.
- A small single-node deployment can evolve without changing domain code.

### Negative

- Three storage components plus the Collector still require backup, upgrades,
  access control, capacity planning, and monitoring.
- Rust telemetry APIs can change while beta and require controlled upgrades.
- Correlation and high-cardinality mistakes can increase storage cost quickly.

### Neutral

- OpenTelemetry is an instrumentation and transport standard, not a guarantee
  of useful dashboards or alert thresholds.
- Grafana remains optional and does not replace the Victoria backends.

## Compliance

- API and worker send OTLP only to the configured Collector endpoint.
- Domain and application crates do not import an exporter or Victoria client.
- Before the release gate is complete, CI must test redaction, bounded labels,
  trace propagation, and disabled-exporter behavior. This foundation does not
  yet contain that CI workflow.
- Collector configuration and local retention live in version control under
  `backend/deploy/observability`; future alerts and dashboards must be added
  there with their validation and ownership rules.
- Production readiness includes an alert-delivery test and an outage test with
  all telemetry backends unavailable.

## References

- [OpenTelemetry Rust](https://opentelemetry.io/docs/languages/rust/)
- [OpenTelemetry Collector](https://opentelemetry.io/docs/collector/)
- [VictoriaMetrics OpenTelemetry integration](https://docs.victoriametrics.com/guides/getting-started-with-opentelemetry/)
- [VictoriaLogs OpenTelemetry ingestion](https://docs.victoriametrics.com/victorialogs/data-ingestion/opentelemetry/)
- [VictoriaTraces OpenTelemetry ingestion](https://docs.victoriametrics.com/victoriatraces/data-ingestion/opentelemetry/)
