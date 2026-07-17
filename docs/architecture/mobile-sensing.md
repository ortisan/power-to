---
id: mobile-sensing
title: Mobile Capture and Road Sensing
sidebar_label: Mobile and Road Sensing
---

# Mobile Capture and Road Sensing

PowerTo will provide Android and iOS applications for in-place civic reporting
and opt-in road surveys. This document separates the accepted product behavior
from the proposed mobile framework and from the experimental pavement-quality
method.

## Capability map

| Capability | Product status | Implementation status |
| --- | --- | --- |
| Android and iOS clients | Accepted | Not implemented |
| Photo/video issue evidence | Accepted | Uses the portable media design |
| Capture point and affected radius/area | Accepted | Requires location/map UX spike |
| Followed-issue geofence notifications | Accepted, permission-dependent | Requires Android/iOS limit and battery spike |
| Explicit road-survey session | Accepted | Feature-flagged pilot |
| Public good/poor road-quality map | Accepted long-term outcome | Requires calibration and aggregation research |
| KMP shared core with Compose/SwiftUI | Proposed | Requires ADR 0014 validation gates |

## Client and backend flow

```text
Android / iOS app
  |
  +-- camera + confirmed location ------> issue draft
  |                                        |
  |                                        +--> Rust API + PostgreSQL/PostGIS
  |                                        +--> direct media upload -> quarantine
  |
  +-- explicit road survey
       |
       +-- native motion + location capture
       +-- encrypted offline batch
       +-- resumable direct upload
                 |
              Rust worker
                 |
       validate -> normalize -> map-match -> score with confidence
                 |
         private observations -> thresholded public road segments
```

The app is an untrusted client. It can guide capture and preserve offline work,
but the server revalidates authorization, bounds, state transitions, object
metadata, batch schemas, timestamps, and idempotency.

## Issue geography versus device geofencing

These are different concepts:

- **Issue geography** is the point and affected area recorded with a report and
  stored in PostGIS. The citizen sees and confirms it before submission.
- **Device geofencing** asks Android or iOS to notify the app when the device
  enters or exits a circular region. It is useful for followed issues or an
  optional reminder, but is delayed, capacity-limited, and permission-dependent.

Never infer a precise issue location from an old background event. Never treat
a geofence transition as proof of presence, residence, or vote eligibility.
Because Android limits active geofences to 100 per app/device user and Apple
limits monitored conditions to 20, the API returns a small prioritized set for
the current user and the app reconciles registrations idempotently.

## Road-survey session

A survey has an explicit start, visible active state, pause, and stop. The app
performs a preflight check:

- consent and permission state;
- available motion sensors and their supported rates;
- location accuracy and timestamp source;
- battery, thermal, free-space, and connectivity policy;
- maximum configured session duration and distance;
- acknowledgement that a driver must not interact with the phone.

The native adapter writes a framed, versioned local format. Each batch contains
session metadata followed by timestamped motion and location samples. A checksum
and atomic rename prevent a partial batch from appearing complete. Upload
progress is persisted so a restart can resume safely.

The exact schema, encoding, compression, encryption, and initial sampling rate
are spike decisions. Fifty samples per second is a reasonable experiment, not a
product constant: iOS exposes 50 Hz system recording in one background API and
Android hardware/rate behavior varies. The method must preserve the actual
observed sampling timestamps instead of assuming a perfect interval.

## Derivation pipeline

The worker uses a versioned, reproducible method:

1. authenticate the capture session and validate schema, checksum, bounds, and
   monotonic ordering;
2. align motion and GNSS time series and retain accuracy/missing-data flags;
3. estimate device orientation and isolate the vertical/road-relevant component
   while removing gravity and obvious vehicle maneuvers;
4. split the trip into bounded windows and reject low-speed, low-accuracy,
   handheld, braking, cornering, and anomalous windows according to policy;
5. map-match accepted windows to versioned road-network segments;
6. compute versioned features and a confidence-scored observation rather than a
   binary good/bad claim;
7. aggregate observations from sufficiently independent trips, devices, and
   time periods;
8. publish segment score, confidence band, sample sufficiency, method version,
   and freshness only after privacy thresholds are met.

Calibration compares device-derived observations with a ground-truth inspection
or accepted reference instrument across road types, speeds, mounts, vehicles,
and device classes. Until that validation exists, UI language uses “vibration
observation” or “possible irregularity,” not “proven bad road” or an official
International Roughness Index.

## Storage and data lifecycle

| Data | Location | Visibility |
| --- | --- | --- |
| Issue point and affected area | PostgreSQL/PostGIS | Generalized public projection; precise value restricted |
| Photo/video metadata | PostgreSQL | Restricted metadata; ready derivative may be authorized publicly |
| Photo/video bytes | R2, S3, or GCS | Private quarantine and sanitized delivery namespaces |
| Raw road-survey batch | R2, S3, or GCS | Private, short retention, processor-only access |
| Precise survey track and quality flags | Restricted PostgreSQL/PostGIS metadata | Never public as a trip |
| Segment observation | PostgreSQL/PostGIS | Restricted until aggregation thresholds pass |
| Aggregated segment quality | Public projection | Public with uncertainty and method version |

Raw trips and precise issue points do not enter logs, traces, metrics, crash
reports, push-notification payloads, or analytics. Operational telemetry uses
bounded outcomes such as capture success, permission class, batch size bucket,
processing result, and method version.

## Abuse, bias, and appeal

- Treat device and operating-system metadata as quality inputs, not proxies for
  citizen credibility or socioeconomic status.
- Weight or filter repeated observations by a published method; do not let one
  account, device, fleet, neighborhood, or vehicle dominate silently.
- Keep the raw-to-derived lineage available to authorized auditors during its
  retention period.
- Provide a way to flag a segment result, add conventional evidence, and request
  professional inspection.
- Publish known limitations and coverage gaps so absence of sensor data is not
  interpreted as good road quality.
- Recompute derived observations when a method changes; never silently compare
  incompatible method versions.

## Safety and store policy

Road surveying is not a reason to encourage phone use while driving. The
feature must be operable before motion starts, remain hands-free, and stop or
pause without requiring interaction. Background location and motion are used
only for an obvious active survey or a separately consented bounded geofence
feature.

Both app stores require truthful purpose descriptions, data minimization,
permission handling, privacy disclosures, and account/data deletion paths. App
review approval is an external constraint and must be proven with an early
store-review spike rather than assumed.

## References

- [ADR 0013: Mobile Capture and Road-Sensing Evidence](decisions/0013-mobile-capture-and-road-sensing.md)
- [ADR 0014: Kotlin Multiplatform with Native Mobile Interfaces](decisions/0014-kotlin-multiplatform-native-mobile.md)
- [User media storage](media-storage.md)
- [Android motion sensors](https://developer.android.com/develop/sensors-and-location/sensors/sensors_motion)
- [Android geofencing](https://developer.android.com/develop/sensors-and-location/location/geofencing)
- [Apple Core Motion](https://developer.apple.com/documentation/coremotion/)
- [Apple geographic condition monitoring](https://developer.apple.com/documentation/corelocation/monitoring-the-user-s-proximity-to-geographic-regions)
- [Apple App Review Guidelines](https://developer.apple.com/app-store/review/guidelines/)
