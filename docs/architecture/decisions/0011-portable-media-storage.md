---
id: 0011-portable-media-storage
title: Portable Media Storage Providers
---

# Portable Media Storage Providers

## Status

ACCEPTED on 2026-07-16 by explicit project-owner decision.

## Context

Citizens will upload photos and videos as evidence. Deployments need a choice
between Cloudflare R2, AWS S3, and Google Cloud Storage without making provider
URLs or SDK types part of the domain. Uploads are untrusted content and can
contain malware, misleading file types, personal metadata, or very large data.

## Decision

Define a media-storage port in the application layer and implement three
provider configurations:

- Cloudflare R2 through an S3-protocol adapter;
- AWS S3 through the same protocol family;
- Google Cloud Storage through a dedicated GCS adapter.

R2 and S3 share implementation code where their behavior is compatible, but
they have separate configuration and provider contract suites. R2 is not
assumed to implement every S3 feature.

The same provider clients may implement a distinct sensor-batch storage port
for ADR 0013. Its private raw-data lifecycle is not part of the media
sanitization or publication contract.

Each media record stores a stable internal ID and a provider-neutral locator:
provider, bucket or namespace, immutable random object key, version when
available, verified SHA-256 digest, byte size, detected media type, and
lifecycle state. Provider URLs and signed URLs are never persisted.

Each environment selects a default provider for new writes. Existing objects
continue to route by their stored provider, allowing a controlled migration or
rollback. The write path does not fan out to several clouds by default.

The API authorizes the request, reserves metadata, and returns a short-lived
direct-upload instruction. Photos use signed PUT; large videos use multipart
or resumable sessions supported by the chosen provider. Uploaded objects enter
a private quarantine area and become readable only after asynchronous
verification and sanitization. See the
[media storage design](../media-storage.md) for the complete flow.

## Consequences

### Positive

- A deployment can choose R2, S3, or GCS through configuration.
- Application containers do not relay large upload bodies.
- Persisted locators survive provider URL and CDN changes.
- Provider-specific behavior remains isolated and contract-tested.

### Negative

- Portability requires maintaining three integration suites and handling
  different multipart, checksum, signing, and error semantics.
- Cross-provider migration is an explicit copy, verify, switch, and delete
  operation.
- A common port cannot expose every provider-specific storage feature.

### Neutral

- Storage choice does not choose a CDN; delivery acceleration is a separate
  outer configuration.
- Object storage durability does not replace application metadata backup,
  retention policy, or restore exercises.

## Compliance

- Buckets are private, public ACLs are blocked, and credentials follow least
  privilege.
- Upload and read grants expire in minutes and are scoped to one immutable key.
- File extension and browser MIME type are never trusted.
- Only sanitized derivatives reach the delivery namespace.
- Contract tests cover upload, finalize, metadata, range read, delete,
  multipart abort, expiry, and provider error mapping for all three providers.
- Logs and traces redact signed URLs, credentials, citizen filenames, and
  object keys.

## References

- [Cloudflare R2 S3 compatibility](https://developers.cloudflare.com/r2/api/s3/api/)
- [Cloudflare R2 presigned URLs](https://developers.cloudflare.com/r2/api/s3/presigned-urls/)
- [AWS S3 presigned uploads](https://docs.aws.amazon.com/AmazonS3/latest/userguide/PresignedUrlUploadObject.html)
- [Google Cloud Storage signed URLs](https://cloud.google.com/storage/docs/access-control/signed-urls)
- [Google Cloud Storage resumable uploads](https://cloud.google.com/storage/docs/resumable-uploads)
