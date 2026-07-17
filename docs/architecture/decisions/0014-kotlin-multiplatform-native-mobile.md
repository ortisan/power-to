---
id: 0014-kotlin-multiplatform-native-mobile
title: Kotlin Multiplatform with Native Mobile Interfaces
---

# Kotlin Multiplatform with Native Mobile Interfaces

## Status

PROPOSED — requires physical-device, background-execution, offline-upload, and
team-productivity spikes on both Android and iOS.

## Context

The Android and iOS applications share issue capture, moderation, voting,
mapping, offline synchronization, and upload workflows. Camera, geofencing,
high-rate motion capture, background execution, and secure storage still depend
on precise platform behavior and store policy.

Two wholly independent applications maximize native control but duplicate
domain, networking, offline state, and synchronization work. A framework that
shares core logic while leaving sensor-heavy UI and drivers native provides a
better initial balance.

## Proposed decision

Use Kotlin Multiplatform (KMP) to share mobile domain types, application use
cases, Ktor/OpenAPI client integration, Kotlin serialization/coroutines, Room
offline state, idempotent outbox/synchronization, and provider-neutral upload
orchestration.

Keep platform interfaces and hardware adapters native:

- Android UI in Jetpack Compose, with CameraX, Android location/geofencing,
  `SensorManager`, foreground services, WorkManager, and the appropriate
  user-initiated transfer mechanism;
- iOS UI in SwiftUI, with AVFoundation, Core Location, Core Motion, background
  `URLSession`, and Apple-protected credential storage.

KMP defines capability ports for evidence capture, current location, geofence
monitoring, road-motion recording, background upload, secure credentials, and
permissions. Kotlin/Swift adapters implement those capabilities without
pretending both operating systems have identical lifecycles or sensor support.

Native adapters own high-rate sampling, timestamps, buffering, background
lifecycle, encryption handoff, and atomic file finalization. They publish only
low-rate status and an opaque finalized batch reference to shared application
logic; individual samples never traverse the UI layer.

Backend civic rules remain authoritative. The mobile shared module may validate
drafts and manage state machines, but it does not become a second source of
truth for eligibility, voting, moderation, or public workflow transitions.

## Target layout

```text
mobile/
  shared/
    domain/          # draft, evidence, road session and upload value types
    application/     # offline use cases and state machines
    data/            # API client, local database, outbox and codecs
    ports/           # camera, location, geofence, motion, upload, credentials
  androidApp/
    ui/              # Jetpack Compose
    adapters/        # CameraX, SensorManager, location and background work
  iosApp/
    ui/              # SwiftUI
    adapters/        # AVFoundation, Core Motion/Location and URLSession
```

Authentication uses OIDC Authorization Code with PKCE in the system browser;
refresh credentials use platform-protected storage. All offline mutations carry
idempotency keys. Media and sensor files use short-lived direct or resumable
upload instructions issued by the Rust API.

The local Room database owns only offline client state. Room schema exports and
migrations are reviewed and tested independently on Android and iOS. They are
completely separate from Atlas, which owns PostgreSQL migrations on the backend.

MapLibre Native is proposed for both platform maps, with tile/style endpoints
in configuration. OpenStreetMap-derived data requires attribution and a hosted
or self-hosted tile service whose policy permits the intended traffic and
offline behavior.

## Alternatives considered

| Option | Benefit | Main cost for PowerTo |
| --- | --- | --- |
| KMP shared core with Compose and SwiftUI | Direct platform APIs plus shared domain/data/sync logic | Two native UIs and Gradle/Xcode expertise |
| Flutter plus owned Kotlin/Swift adapters | One UI and a clear federated plugin model | Adds Dart and a runtime bridge while critical features remain native |
| React Native | Reuses TypeScript skills from the web application | Critical features still need native modules; current Swift modules add native glue |
| Separate Kotlin and Swift applications | Maximum platform independence and control | Highest duplication and cross-platform behavior drift |

## Consequences

### Positive

- Sensor, camera, background, and accessibility behavior use first-party
  platform APIs directly.
- Offline and synchronization rules are implemented once and tested in shared
  code.
- Native UI follows each platform's accessibility, permission, and lifecycle
  conventions.
- Platform capability differences remain explicit adapters.

### Negative

- The team maintains Jetpack Compose and SwiftUI interfaces and both build
  toolchains.
- Kotlin/Swift interop and shared-module API design require discipline.
- UI parity needs behavior contracts and duplicated end-to-end scenarios.

### Neutral

- This proposal does not share Rust backend domain code with mobile.
- A failed spike may choose Flutter or fully separate apps without changing ADR
  0013's product, evidence, safety, or privacy boundaries.

## Validation gates

1. Record synchronized motion and location on representative devices at a
   bounded sample rate without dropping or reordering samples.
2. Continue an explicitly active survey under permitted background behavior
   with clear system and in-app indication.
3. Survive interruption, process termination, low storage, denied permissions,
   and lost connectivity without corrupting a batch.
4. Upload large media and sensor batches resumably and idempotently using a
   backend-issued provider-neutral plan.
5. Meet screen-reader, dynamic text, contrast, keyboard/switch, and reduced
   motion expectations on both platforms.
6. Demonstrate acceptable battery, heat, storage, and data usage for the pilot
   session length.
7. Prove raw location and sensor values are absent from crash reports,
   analytics, logs, and OpenTelemetry signals.
8. Measure the cost of implementing one representative end-to-end screen and
   one shared offline workflow across both UIs before accepting KMP.

## References

- [Android support for Kotlin Multiplatform](https://developer.android.com/kotlin/multiplatform)
- [Kotlin Multiplatform recommended project structure](https://kotlinlang.org/docs/multiplatform/multiplatform-project-recommended-structure.html)
- [Android CameraX](https://developer.android.com/media/camera/camerax)
- [Room for Kotlin Multiplatform](https://developer.android.com/kotlin/multiplatform/room)
- [Ktor multiplatform client](https://ktor.io/docs/client-create-new-application.html)
- [MapLibre Native](https://maplibre.org/projects/native/)
- [OpenStreetMap tile usage policy](https://operations.osmfoundation.org/policies/tiles/)
- [Flutter platform-specific code](https://docs.flutter.dev/platform-integration/platform-channels)
- [React Native Swift native modules](https://reactnative.dev/docs/the-new-architecture/turbo-modules-with-swift)
