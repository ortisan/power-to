---
id: 0008-relational-vote-ledger
title: Relational, Tamper-Evident Vote Ledger Before Blockchain
---

# Relational, Tamper-Evident Vote Ledger Before Blockchain

## Status

PROPOSED

## Context

Votes must be concurrency-safe, private, auditable, and resistant to silent
administrator changes. ADR 0002 proposes Hyperledger Fabric, DIDs, consortium
nodes, chaincode, and sharding before PowerTo has a threat model, validator
organizations, network governance, or an implemented voting rule. Fabric also
does not provide official Rust application or chaincode APIs, which conflicts
with ADR 0004.

Blockchain does not by itself prove a person's eligibility, preserve ballot
secrecy, prevent coercion, define fair geographic weighting, or make a validator
consortium institutionally independent.

## Decision

For the first release, record votes in PostgreSQL using:

- an immutable, versioned poll and rule set;
- a poll-scoped voter pseudonym, separated from identity evidence;
- a database uniqueness constraint for one effective vote per poll and
  pseudonym;
- a serializable transaction that writes the ballot, append-only audit event,
  and outbox record together;
- opaque citizen receipts and privacy-preserving public aggregates;
- hash-linked audit events and signed checkpoints copied to an independently
  controlled, retention-locked location and published for verification.

Changing a vote, if permitted by policy, appends a superseding event. It never
updates civic history in place.

Reconsider a permissioned ledger only after independent organizations commit
to operating validators and agree on membership, keys, upgrades, incident
handling, dispute resolution, privacy, retention, and exit procedures. The
first open-source Rust candidate for that evaluation is Hyperledger Iroha 2,
which targets permissioned and consortium deployments under Apache 2.0. The
evaluation must still compare Iroha with a transparency log and independently
witnessed checkpoints under an explicit threat model.

## Consequences

### Positive

- Vote state and audit evidence retain simple, testable transaction semantics.
- The full backend remains Rust and the MVP has no consortium infrastructure.
- Signed external checkpoints make database-history rewrites detectable.
- The architecture can validate voting rules and user trust before choosing a
  distributed ledger.

### Negative

- The platform operator still coordinates transactions and availability.
- Hash chains without independently retained checkpoints can be rewritten by a
  sufficiently privileged operator.
- Key management, public verification tooling, and independent witnessing are
  still non-trivial security work.

### Neutral

- This is civic prioritization architecture, not certification for binding
  governmental elections.
- A later ledger can consume the same versioned domain events if governance
  justifies it.

## Compliance

- Update and delete privileges are denied on ballot and civic audit history to
  the application database role.
- Concurrency tests prove duplicate effective votes cannot commit.
- Receipt and checkpoint verification has public test vectors and independent
  implementation guidance.
- Security tests demonstrate that public data cannot link a ballot to a person
  or link the same pseudonym across polls.
- No blockchain component enters the release path while ADR 0002 is on hold.

## References

- [Hyperledger Fabric Gateway APIs](https://hyperledger-fabric.readthedocs.io/en/latest/gateway.html)
- [Hyperledger Fabric supported APIs](https://hyperledger-fabric.readthedocs.io/en/latest/sdk_chaincode.html)
- [Hyperledger Iroha repository and license](https://github.com/hyperledger-iroha/iroha)
- [Linux Foundation announcement of Iroha 2.0](https://www.lfdecentralizedtrust.org/announcements/lf-decentralized-trust-announces-new-identity-project-hyperledger-iroha-2.0-release-and-line-up-of-new-subprojects-and-labs)
