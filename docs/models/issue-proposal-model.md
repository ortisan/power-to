# Issue Proposal Model

## Purpose and status

This document defines the product-level information needed to report a civic
problem. It is a working model for the MVP, not a finalized API or database
schema. Field limits, categories, moderation policy, geographic eligibility,
and public-location precision remain versioned jurisdiction policy.

An issue proposal records an observed problem and its affected area. It does
not promise funding, procurement, or execution, and it does not require the
submitter to design a solution.

## Modeling principles

- Capture the problem, impact, and desired outcome without forcing a proposed
  technical solution.
- Keep authentication and private contact data outside the public issue
  record. An OIDC subject links the submission to its account internally.
- Store the confirmed issue geometry separately from the privacy-safe geometry
  or label published to other users.
- Treat photos, videos, documents, links, and road-survey observations as
  evidence with provenance and moderation state, not as proof by themselves.
- Configure categories and participation rules by jurisdiction instead of
  hard-coding street, district, city, or state weights in this model.
- Redirect emergencies and immediate threats to the responsible emergency
  channel; PowerTo is not an emergency-response service.

## Proposal structure

### Problem

| Field | Required | Visibility | Purpose |
| --- | --- | --- | --- |
| `title` | yes | public after moderation | Concise, neutral description of the problem |
| `category_id` | yes | public | Jurisdiction-configured category reference |
| `summary` | yes | public | Short explanation suitable for lists and maps |
| `problem_statement` | yes | public after moderation | Observable facts, duration, frequency, and current condition |
| `affected_community` | yes | public after moderation | People or public services affected, without naming private individuals |
| `desired_outcome` | yes | public after moderation | What improvement would mean, without prescribing procurement |
| `proposed_solution` | no | public after moderation | Submitter suggestion clearly labeled as non-binding |
| `previous_attempts` | no | public after moderation | Existing public protocols or attempts, with private data removed |
| `time_context` | no | public after moderation | Relevant recurrence, season, or non-emergency timing |

Text limits, supported languages, and content policy belong to the versioned
submission policy. The UI must explain those limits before upload or submit.

### Geography

| Field | Required | Visibility | Purpose |
| --- | --- | --- | --- |
| `observed_geometry` | yes | restricted by policy | User-confirmed point or affected area in WGS 84 |
| `geometry_source` | yes | private operational metadata | Map selection, current device position, or geocoded search result |
| `public_geometry` | derived | public | Policy-approved representation that does not expose a person's private location |
| `jurisdiction_id` | derived and confirmed | public | Responsible or affected administrative jurisdiction |
| `location_label` | no | public after moderation | Safe landmark, road segment, or area description |

Device location is a convenience input, not evidence of residence, identity,
or eligibility. A submitter must confirm the point or area on the map before
submission. Moderators may correct geography while preserving who changed it,
why, and which previous value existed.

### Supporting evidence

| Field | Required | Visibility | Purpose |
| --- | --- | --- | --- |
| `media` | no | restricted until processed | Photo/video media IDs uploaded through the quarantine workflow |
| `documents` | no | restricted until processed | Supporting document media IDs when jurisdiction policy allows them |
| `external_links` | no | public after moderation | References to relevant public sources |
| `estimated_impact` | no | public after moderation | Clearly labeled submitter estimate and its basis |
| `road_observations` | no | aggregated only | References to consented road-survey batches processed separately |

Uploads are private by default. The media pipeline verifies size and type,
scans content, removes metadata where applicable, creates safe derivatives,
and exposes only authorized results. Original filenames, signed provider URLs,
and storage SDK types are not part of this model.

A raw accelerometer stream or a single trip must never appear as a public road
quality score. Public road segments require the calibrated aggregation,
uncertainty, and privacy thresholds described in
[mobile capture and road sensing](../architecture/mobile-sensing.md).

### Submitter and consent

| Field | Required | Visibility | Purpose |
| --- | --- | --- | --- |
| `submitted_by` | yes | private | Internal account reference derived from the authenticated OIDC subject |
| `organization_id` | no | public only when authorized | Verified organization represented by the submitter |
| `notification_preferences` | no | private | Follow-up channels and state changes requested by the user |
| `public_attribution_consent` | yes | private decision | Whether an approved display name may be shown; default is anonymous |
| `evidence_rights_attestation` | when evidence exists | private audit | Confirmation that the user may submit the material |
| `privacy_notice_version` | yes | private audit | Notice accepted for this submission |

The proposal form does not request a name or email for publication. Contact
attributes remain in the identity/profile boundary and are disclosed only to
authorized workflows. Withdrawing public attribution does not delete the
private accountability record or a legally required audit event.

## Example

```yaml
title: "Deep pothole on the eastbound bus lane"
category_id: "road-surface"
summary: "A recurring pothole forces buses and bicycles into the adjacent lane."
problem_statement: >
  The pavement has an open depression on the bus lane. It remains present on
  repeated observations and collects water after rain.
affected_community: "Bus passengers, cyclists, drivers, and nearby pedestrians."
desired_outcome: "Restore a level, safely usable road surface and verify the repair."
observed_geometry: "user-confirmed point on the map"
location_label: "Eastbound bus lane near the public library"
media:
  - "opaque-media-reference"
public_attribution_consent: false
```

The example deliberately omits coordinates, personal contact details, storage
URLs, and a procurement prescription. API fixtures must use synthetic data.

## Submission and moderation flow

1. Save an encrypted local draft when the client is offline.
2. Validate required fields, acknowledgements, evidence limits, and confirmed
   geography before upload.
3. Upload evidence to private quarantine and submit only opaque media
   references with the proposal command.
4. Assign an opaque public tracking reference and place the proposal in
   moderation; submission does not make it public.
5. Detect likely duplicates and let a moderator link, merge, request
   clarification, publish, reject, or escalate the proposal under a versioned
   policy.
6. Record every moderation transition, reason, actor, policy version, and
   privacy-safe audit receipt.
7. Notify the submitter without placing message content or contact attributes
   in telemetry.

An issue may move through states such as:

```text
draft -> submitted -> in_moderation -> needs_clarification
      -> published | rejected -> appeal/review when policy allows
```

Published issues may later enter a separately versioned voting period. Voting
eligibility, weights, quorum, thresholds, ties, and withdrawal rules are not
encoded in the proposal model. The voting use case stores an eligibility
snapshot explaining which rule version applied without publishing a citizen's
address or ballot.

## MVP boundary

A highly supported issue can become prioritized and tracked, but the MVP does
not automatically start cost analysis, choose a service provider, approve a
budget, or create a government obligation. Feasibility, proposals,
procurement, delivery, payments, and citizen acceptance are later product
phases that require a real jurisdiction and legal governance.

Blockchain is also outside this submission flow. The initial integrity model
uses PostgreSQL transactions, append-oriented audit evidence, signed
checkpoints, and privacy-safe receipts as described in
[ADR 0008](../architecture/decisions/0008-relational-vote-ledger.md).

## Safety and privacy checks

- Reject secrets, authentication tokens, signed URLs, and provider object keys
  from public fields.
- Warn users not to upload faces, license plates, home addresses, minors, or
  unrelated bystanders when they are unnecessary to document the problem.
- Strip image/video location metadata from public derivatives while retaining
  only policy-authorized evidence provenance.
- Never emit proposal text, precise geometry, media references, identity data,
  or raw sensor samples to logs, metrics, or traces.
- Provide accessible map alternatives and a non-map way to describe the area.
- Preserve a clear emergency redirect and moderation route for illegal,
  dangerous, defamatory, or personally identifying content.
