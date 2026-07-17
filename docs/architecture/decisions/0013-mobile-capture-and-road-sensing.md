---
id: 0013-mobile-capture-and-road-sensing
title: Mobile Capture and Road-Sensing Evidence
---

# Mobile Capture and Road-Sensing Evidence

## Status

ACCEPTED on 2026-07-16 by explicit project-owner decision.

## Context

PowerTo needs Android and iOS applications so citizens can capture photos,
videos, and the affected geographic area at the place where a problem occurs.
The applications will also use motion and location sensors during an explicit
road-survey session to map indications of good and poor pavement quality.

Accelerometer measurements are affected by phone orientation and mounting,
vehicle suspension, speed, braking, device hardware, sensor availability, and
driving behavior. A single vibration measurement cannot by itself prove road
condition or justify a civic decision.

## Decision

Android and iOS are first-class PowerTo clients. They support two distinct
workflows:

1. **Issue capture:** take or select a photo/video, capture an accurate point,
   and let the citizen confirm an affected radius or area before submission.
2. **Road survey:** an opt-in, visibly active session records synchronized
   motion and location samples for later quality analysis.

The domain references evidence IDs, not mobile SDK types or raw sensor arrays.
Photo and video processing follows ADR 0011. Road-survey metadata lives in
PostgreSQL/PostGIS; compressed raw sample batches live encrypted in the
configured object-storage provider and are private.

A survey records monotonic and wall-clock timestamps, acceleration or linear
acceleration, attitude/rotation information when available, location, speed,
heading, horizontal accuracy, sensor capabilities, app/method version, and a
pseudonymous device-class profile. It does not record audio. Collection stops
when the user stops the survey, the safety policy fails, or a configured time,
distance, storage, temperature, or battery limit is reached.

The processing pipeline calibrates and resamples signals, compensates for
gravity and orientation, rejects low-quality windows, map-matches observations
to road segments, extracts versioned features, and produces confidence-scored
segment observations. Public road-quality maps aggregate multiple eligible
observations and expose the method version, sample sufficiency, uncertainty,
and update time. Good-quality observations are retained in the aggregate so
the map is not merely a catalog of complaints.

Sensor observations are supporting evidence. They do not automatically create
an issue, determine its truth, change a vote, rank a neighborhood, or certify a
formal pavement index. Promotion to a calibrated road-quality measure requires
ground-truth data, a published methodology, bias analysis across devices and
vehicles, and independent validation.

## Location and geofencing

Precise location is captured just in time and can always be corrected manually.
Background location is not required merely to submit an issue.

Operating-system geofences are reserved for a bounded set of followed or
nearby issues. The backend sends a prioritized working set instead of trying to
register every issue: Android currently allows 100 active geofences per app and
device user, while Apple condition/region monitoring allows 20. Entry and exit
events are delayed and advisory; they are never proof that a person visited a
problem or is eligible to vote.

## Safety and privacy rules

- A road survey requires explicit, revocable consent and a persistent visual
  indication. There is no covert or indefinite background collection.
- The app instructs a driver not to handle the phone. Collection is intended
  for a passenger or a securely mounted device and must comply with local law.
- Permissions are requested in context and separately for camera, media,
  foreground location, background location, and motion. A manual location and
  non-sensor path remains available where practical.
- Offline batches are encrypted and integrity-protected until upload. Upload is
  resumable and idempotent, and local data is deleted after verified receipt
  according to policy.
- A precise trip is personal location data. Raw routes are never public, never
  used as telemetry attributes, and never reused to establish residence or
  voting eligibility.
- Public segments require spatial and temporal aggregation thresholds that
  prevent revealing a person's route, home, workplace, or repeated habits.
- Users can inspect survey status, revoke future collection, and request
  deletion subject to the documented legal basis and retention policy.
- Server checks detect impossible speeds, replayed sessions, timestamp gaps,
  duplicate samples, and incompatible sensor configurations without claiming
  perfect fraud prevention.

## Consequences

### Positive

- Evidence can be captured at the problem location with an offline-capable
  guided workflow.
- Repeated surveys can reveal both good and poor road segments with published
  confidence rather than anecdote alone.
- Raw high-volume data stays out of PostgreSQL and public APIs.
- Platform limits and privacy constraints are explicit before implementation.

### Negative

- Reliable road assessment requires physical-device testing, calibration,
  ground truth, map matching, and ongoing algorithm governance.
- Motion and precise-route data materially increase privacy, security, battery,
  store-review, and retention obligations.
- Android and iOS differ in sensor delivery, background execution, permission,
  and geofence behavior.

### Neutral

- Choosing Android and iOS does not yet accept a cross-platform UI framework.
- Mobile sensor evidence complements, but does not replace, photos, videos,
  citizen reports, or professional inspection.

## Compliance

- The road-survey feature is feature-flagged until a safety, privacy, and data
  protection review is approved for the pilot jurisdiction.
- A published schema and method version make every derived observation
  reproducible from authorized raw data.
- CI tests codecs and algorithms; release testing includes representative real
  Android and iOS devices, vehicles, mounts, speeds, and known road segments.
- Public APIs expose aggregates and uncertainty only; authorization tests prove
  that raw routes and device identifiers cannot enter public projections.
- Sensor, location, media, and consent events are covered by explicit retention
  and deletion tests.

## References

- [Android motion sensors](https://developer.android.com/develop/sensors-and-location/sensors/sensors_motion)
- [Android geofencing](https://developer.android.com/develop/sensors-and-location/location/geofencing)
- [Android background location](https://developer.android.com/develop/sensors-and-location/location/background)
- [Apple Core Motion](https://developer.apple.com/documentation/coremotion/)
- [Apple raw accelerometer events](https://developer.apple.com/documentation/coremotion/getting-raw-accelerometer-events)
- [Apple geographic condition monitoring](https://developer.apple.com/documentation/corelocation/monitoring-the-user-s-proximity-to-geographic-regions)
- [Apple background location](https://developer.apple.com/documentation/corelocation/handling-location-updates-in-the-background)
