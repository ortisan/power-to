---
id: media-storage
title: User Media Storage
sidebar_label: User Media Storage
---

# User Media Storage

This design covers photos and videos uploaded by citizens. Application static
assets continue to be built and deployed with the web application; they do not
use this workflow. Raw mobile road-survey batches use the same configured object
providers through a separate sensor-evidence port and lifecycle; they are not
treated as playable media or sent through the image/video publication pipeline.

## Goals

- Select Cloudflare R2, AWS S3, or Google Cloud Storage per deployment.
- Keep provider SDKs and identifiers outside the domain and public API.
- Avoid relaying large upload bodies through the Rust API.
- Treat every upload as hostile until it is verified and sanitized.
- Support provider migration without changing civic records or public URLs.
- Honor retention, legal hold, and deletion rules without losing auditability.

## Clean Architecture boundary

The application layer defines capabilities, not an S3-shaped interface:

- reserve an immutable object locator;
- create a short-lived upload instruction;
- inspect/finalize an upload;
- open a bounded read stream for processing;
- write a sanitized derivative;
- create a short-lived delivery instruction;
- abort an incomplete multipart upload;
- delete an object idempotently.

The Cloudflare R2 and AWS S3 adapters can share SigV4 and multipart code behind
an internal S3-protocol client. They remain distinct configurations with
independent contract tests because R2's compatibility surface is not identical
to S3. Google Cloud Storage uses its own adapter and resumable-upload behavior.

Provider errors map to a small application taxonomy such as unavailable,
expired grant, missing object, conflict, quota exceeded, and rejected request.
Raw SDK errors never cross the adapter boundary.

The provider adapters may share signing, multipart/resumable transfer,
metadata, stream, and deletion code with a `SensorBatchStore` implementation.
The application ports remain separate: media produces sanitized derivatives,
while a sensor batch remains private input to a versioned analysis method.

## Persisted model

PostgreSQL owns media metadata and lifecycle state. An initial record contains:

| Field | Purpose |
| --- | --- |
| `media_id` | Stable internal ID referenced by civic records |
| `provider` | Stable value: `cloudflare-r2`, `aws-s3`, or `google-cloud-storage` |
| `namespace` | Configured bucket/container alias, not a public URL |
| `object_key` | Random immutable key with no citizen filename or PII |
| `provider_version` | Object generation/version when the provider exposes one |
| `purpose` | Evidence photo, evidence video, avatar, or future bounded purpose |
| `state` | Current lifecycle state |
| `declared_size/type` | Untrusted request hints used for early rejection |
| `verified_size/type` | Values detected after upload |
| `sha256` | Digest computed by the trusted processing path |
| `retention_until` | Policy decision for deletion eligibility |
| timestamps and actor IDs | Private operational audit data |

Do not persist a signed URL, provider endpoint, original filename, or CDN URL.
Do not use ETag as a portable content checksum: multipart and provider semantics
differ.

## Upload and publication flow

```text
Browser -> Rust API: request upload (purpose, declared type and size)
API -> PostgreSQL: reserve media ID, provider locator, quota and expiry
API -> Browser: short-lived direct-upload instruction
Browser -> Provider quarantine: PUT or multipart/resumable upload
Browser -> Rust API: finalize media ID
API -> Outbox: enqueue verification
Worker -> Provider: inspect and stream untrusted object
Worker: detect type, enforce limits, scan, strip metadata, transform
Worker -> Delivery namespace: write sanitized derivatives
Worker -> PostgreSQL: mark ready or rejected; append audit outcome
Browser -> API: request authorized delivery
API -> Browser: short-lived signed URL or stable application delivery URL
```

Use signed PUT for ordinary photos. Videos above the configured threshold use
multipart upload for R2/S3 or a resumable upload session for GCS. The upload
instruction includes the exact HTTP method, required headers, maximum size,
expiry, and opaque media ID. It is valid for one immutable key only.

Finalize is idempotent. It never trusts a browser claim that bytes arrived; the
worker reads provider metadata and the object itself. A reconciliation job
finds expired reservations, abandoned multipart sessions, and uploaded objects
whose finalize message was lost.

## State machine

```text
reserved -> uploading -> uploaded -> verifying -> ready
    |           |            |          |
    +-----------+------------+----------+-> rejected
                         ready -> deleting -> deleted
```

`quarantined` is an operational substate for suspicious content needing manual
handling. State transitions are compare-and-set and idempotent. A rejected or
deleted media object remains as a minimal non-PII audit tombstone; its bytes do
not remain unless a documented legal hold requires them.

## Verification and transformation

The trusted worker performs, in order:

1. enforce provider-reported byte limits and download/decompression budgets;
2. detect file type from content signatures, not extension or browser MIME;
3. reject unsupported codecs, malformed containers, polyglots, and dimension or
   duration bombs;
4. scan with the selected malware engine in a sandbox with time and memory
   limits;
5. decode and re-encode images, remove EXIF and other metadata, and generate
   bounded thumbnails;
6. probe and transcode videos with a sandboxed FFmpeg profile, remove metadata,
   and generate preview images;
7. compute a trusted SHA-256 digest and write derivatives to a private delivery
   namespace;
8. publish only after every required derivative is complete.

Original files remain private in quarantine for the shortest policy-approved
period. Public pages use sanitized derivatives, never original citizen uploads.

## Security and privacy controls

- Block public bucket access and ACLs; grant each workload only the actions and
  prefixes it needs.
- Use provider-managed encryption by default and customer-managed keys only
  with an accepted key-management and recovery design.
- Configure exact CORS origins, methods, and headers; never use wildcard origins
  with authenticated access.
- Expire upload and delivery grants in minutes. A resumable-session URL is a
  bearer secret and receives the same redaction and expiry treatment.
- Enforce per-user and per-jurisdiction quotas, rate limits, maximum dimensions,
  duration, and byte sizes before and after upload.
- Never log signed URLs, credentials, raw object keys, original filenames, or
  free-form media metadata.
- Separate quarantine and delivery credentials; compromise of the web delivery
  path must not grant writes to quarantine.
- Audit grants and lifecycle decisions, not every byte-range request.

## Provider selection and migration

Configuration chooses the default provider for new objects. Reads and deletes
route by the provider stored on each media record, so a deployment can safely
contain objects in more than one provider during migration.

Migration is an explicit worker workflow:

1. copy the original or approved derivative to a new immutable key;
2. verify size and trusted digest at the destination;
3. atomically switch the database locator while retaining an audit event;
4. observe a cooling period and exercise reads from the new provider;
5. delete the old object according to retention policy.

There is no synchronous cross-cloud dual write. If disaster-recovery replication
becomes necessary, select and test it separately with clear RPO, cost, and data
residency requirements.

## Provider capability matrix

| Capability | Cloudflare R2 | AWS S3 | Google Cloud Storage |
| --- | --- | --- | --- |
| Adapter | S3 protocol, R2 profile | S3 protocol, AWS profile | Native GCS profile |
| Small direct upload | Presigned PUT | Presigned PUT | V4 signed PUT |
| Large video | S3 multipart API | S3 multipart API | Resumable upload session |
| Integrity | Trusted worker SHA-256 | Trusted worker SHA-256; SigV4 checksums where configured | Trusted worker SHA-256; provider checks are supplemental |
| Delivery | Signed GET or application CDN domain | Signed GET; optional CloudFront | Signed GET; optional Cloud CDN |
| Main portability concern | Partial S3 feature compatibility | AWS-specific policies and services | Different signing, generations, and resumable protocol |

The common contract is the minimum required for PowerTo safety, not the
intersection of every cloud feature. Provider-specific optimizations remain
inside adapters and cannot change civic behavior.

## Contract and failure tests

Run the same black-box suite against disposable or isolated test buckets for
all three providers. Cover:

- upload grant scope, required headers, expiry, and overwrite prevention;
- multipart/resumable completion and abort;
- metadata inspection, range reads, digest verification, and missing objects;
- retry classification, timeouts, throttling, and idempotent finalize/delete;
- denied public access and denied cross-prefix credentials;
- provider switch with old-object reads and a copy/verify migration;
- redaction of grants, filenames, keys, and provider errors from telemetry.

Local emulation may accelerate development, but provider contract tests must
run against each real service before that adapter is declared production-ready.

## References

- [ADR 0011: Portable Media Storage Providers](decisions/0011-portable-media-storage.md)
- [Cloudflare R2 S3 compatibility](https://developers.cloudflare.com/r2/api/s3/api/)
- [Cloudflare R2 multipart uploads](https://developers.cloudflare.com/r2/objects/multipart-objects/)
- [AWS S3 presigned uploads](https://docs.aws.amazon.com/AmazonS3/latest/userguide/PresignedUrlUploadObject.html)
- [AWS S3 multipart upload](https://docs.aws.amazon.com/AmazonS3/latest/userguide/mpuoverview.html)
- [Google Cloud Storage signed URLs](https://cloud.google.com/storage/docs/access-control/signed-urls)
- [Google Cloud Storage resumable uploads](https://cloud.google.com/storage/docs/resumable-uploads)
