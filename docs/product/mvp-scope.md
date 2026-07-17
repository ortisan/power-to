---
id: mvp-scope
title: Product Scope and MVP
sidebar_label: Product Scope and MVP
---

# Product Scope and MVP

## Product intent

PowerTo is a civic accountability platform. It turns a locally observed problem
into a public, traceable process:

```text
report -> moderation -> local participation -> prioritization
       -> feasibility and proposals -> approval -> delivery
       -> citizen evaluation -> correction or resolution
```

The long-term scope is broader than an issue-reporting application. It includes
participatory prioritization, public-sector collaboration, provider selection,
delivery tracking, citizen acceptance of completed work, and privacy-safe maps
of infrastructure observations collected by mobile devices.

## Actors

| Actor | Main responsibility |
| --- | --- |
| Visitor | Inspect public issues, rules, decisions, and aggregated results |
| Citizen | Submit an issue, provide evidence, vote when eligible, and follow progress |
| Road survey participant | Opt into a visible mobile session that contributes motion/location observations without making a professional road-quality claim |
| Moderator | Review content, request clarification, publish, reject, or suspend an issue |
| Government representative | Assess feasibility, approve resources, and record an official response |
| Service provider | Maintain a verified profile, submit a proposal, and report delivery |
| Citizen evaluator | Assess a delivered service and request correction when the acceptance rule is not met |
| Auditor | Inspect rules, privileged actions, vote evidence, and the public transparency log |
| Platform operator | Operate the service without being able to silently rewrite civic history |

One person may hold more than one role. Authorization must be scoped to a
jurisdiction and must keep moderation, procurement, and audit duties separate.

## Recommended first release

The first release should validate the shortest useful civic loop:

1. Authenticate a citizen through an OpenID Connect provider.
2. Submit a structured, geolocated issue from web, Android, or iOS, with mobile
   photo/video capture and a citizen-confirmed affected point or area.
3. Keep precise location and personal data private while publishing a safe,
   generalized location.
4. Moderate the submission with reasons and an appeal-ready history.
5. Publish searchable issue pages and a map designed for low-bandwidth and
   accessible use.
6. Determine voting eligibility from a versioned geographic rule.
7. Accept at most one effective vote per eligible person and poll.
8. Publish aggregated results, the rule version, and a tamper-evident timeline.
9. Notify participants about material state changes.
10. Run a feature-flagged road-sensing pilot that validates consent, safety,
    offline capture, device variability, and aggregation without influencing
    voting or automatically declaring a road defective.

This slice validates whether communities will report, moderate, participate,
and trust the resulting prioritization before the project implements public
procurement workflows.

## Explicitly outside the first release

- Automated provider selection or award of a public contract
- Government budget execution or payments
- Provider delivery scoring and citizen rework cycles
- Blockchain, decentralized identity, or a validator consortium
- Microservices, Kubernetes, Kafka, or a dedicated search cluster
- A single fixed formula for every jurisdiction
- Continuous or covert background location collection
- Registering every public issue as an operating-system geofence
- Treating one phone trip or an uncalibrated vibration score as authoritative
  proof of pavement quality

These are not rejected product capabilities. They are deferred until the first
loop is validated and the legal and institutional authority to operate them is
clear.

## Lifecycle

The initial issue lifecycle is deliberately smaller than the long-term flow:

```text
draft
  -> submitted
  -> under_review
  -> published
  -> voting_open
  -> prioritized | not_prioritized
  -> acknowledged | archived
```

Exceptional transitions include `changes_requested`, `rejected`, `suspended`,
and `withdrawn`. Every transition records the actor, time, reason, and rule or
policy version. States are changed by explicit commands; arbitrary status edits
are not allowed.

The later procurement and delivery lifecycle will be modeled separately. It
must not be encoded as extra flags on the issue aggregate.

## Product principles that constrain architecture

- **Local voice with explainable rules:** proximity, quorum, weights, and
  deadlines are versioned policy, never hard-coded constants.
- **Privacy by default:** exact addresses, identity evidence, email addresses,
  and individual ballots are not public records.
- **Public process:** public outputs expose decisions, reasons, rule versions,
  aggregate totals, and integrity evidence without exposing a person's vote.
- **Human appeal:** moderation and future procurement decisions require a
  reason and a path for review.
- **Inclusive access:** WCAG 2.2 AA, keyboard access, screen readers, low data
  use, responsive layouts, and localization are release requirements.
- **Institutional neutrality:** a jurisdiction can configure its rules, but
  historical results remain tied to the rule version used at the time.
- **Sensor evidence with uncertainty:** road observations disclose method,
  confidence, sample sufficiency, and limitations; missing data is not evidence
  of good quality.
- **Safe, voluntary collection:** a mobile road survey has explicit start/stop,
  visible indication, purpose-bound permissions, and a hands-free safety model.

## Decisions required before a public pilot

The following are product or governance decisions, not implementation details:

1. Country, first jurisdiction, governing law, and the legal operator.
2. Whether PowerTo is advisory or has formal authority in public procurement.
3. What proves identity and geographic eligibility without publishing a home
   address.
4. The first voting policy: geographic levels, weights, quorum, duration,
   ties, vote changes, and appeals.
5. Moderation policy, emergency escalation, abuse handling, and response time.
6. Personal-data purpose, consent or other legal basis, retention, deletion,
   incident response, and data-controller responsibilities.
7. Public aggregation thresholds that reduce re-identification in small areas.
8. Pilot success measures and expected volume, availability, RTO, and RPO.
9. Mobile location/motion purpose, consent, background behavior, raw-route
   retention, deletion, store disclosure, and driver/passenger safety policy.
10. Road-quality ground truth, device/vehicle calibration protocol, public
    aggregation threshold, uncertainty language, and appeal/inspection path.

Until those decisions are made, architecture documents use Brazil's LGPD and a
small-team, web-and-mobile pilot as conservative working assumptions. They do
not claim legal compliance by themselves.
