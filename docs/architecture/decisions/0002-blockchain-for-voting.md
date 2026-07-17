---
id: 0002-blockchain-for-voting
title: Blockchain for Voting
---

# Blockchain for Voting

## Status

ON HOLD — excluded from the MVP pending a voting threat model, independent
validator governance, and comparison with ADR 0008.

## Context

The PowerTo platform is designed to enable democratic decision-making through community voting on issues. The voting process requires a high level of trust, transparency, and security to ensure:

1. Votes are accurately recorded and cannot be tampered with
2. The voting process is transparent and verifiable by all participants
3. Trust in the system is maintained even without trusting individual operators
4. The integrity of the prioritization process is protected from manipulation

Traditional database systems, while efficient, present several challenges for voting systems:

- They are typically centralized, creating single points of failure
- Vote records can be modified by administrators, potentially without leaving traces
- Verification of vote integrity requires trust in the system operators
- It's difficult to provide public verifiability while maintaining voter privacy

Key questions to address:
- How can we ensure votes are recorded immutably and transparently?
- How can we maintain trust in the voting process without requiring trust in central authorities?
- How can we provide public verifiability while preserving voter privacy?
- What technical approach would best balance security, transparency, and performance for the voting system?

## Decision

This record makes no current implementation decision while it is on hold. ADR
0008 is the active proposal for the first voting ledger. Hyperledger Iroha 2 is
the first open-source Rust candidate for a later permissioned-ledger spike if
independent validator governance becomes real.

The following Fabric design is retained only as the original proposal and must
not be treated as the target architecture.

### Historical Blockchain-Based Voting Proposal

The original proposal would use a permissioned Hyperledger Fabric network to
handle voting:

- **Architecture**: A consortium blockchain where multiple trusted entities (e.g., community organizations, local government) operate validator nodes
- **Smart Contracts**: Implement voting logic as smart contracts (chaincode) that:
  - Validate voter eligibility
  - Record votes immutably
  - Prevent double-voting
  - Calculate and update issue rankings based on voter proximity to the issue location
  - Prioritize votes from citizens who share the same street, district, or state as the reported issue
  - Enforce voting rules and timeframes
- **Identity Management**: Use a decentralized identity (DID) system for voter authentication while preserving privacy
- **User Experience**: Abstract blockchain complexity from end-users through intuitive interfaces
- **Scalability**: Implement a sharding approach to handle high transaction volumes during peak voting periods

### Audit Logging Approach

For audit logging and other system records, we will use traditional database systems with enhanced security measures:

- **Secure Databases**: Use properly configured and secured database systems with access controls
- **Cryptographic Verification**: Implement cryptographic signatures for log entries to detect tampering
- **Regular Backups**: Maintain secure, regular backups of audit logs
- **Access Controls**: Implement strict access controls and monitoring for audit log access
- **Separation of Duties**: Ensure no single administrator can modify logs without detection

### Historical Implementation Approach

The original proposal described these phases:

- **Phase 1**: Develop a proof of concept for the blockchain voting system with simulated data
- **Phase 2**: Implement the core voting blockchain with basic functionality
- **Phase 3**: Develop the secure audit logging system using traditional database technology
- **Phase 4**: Integrate both systems with the rest of the PowerTo platform
- **Phase 5**: Conduct security audits and performance testing
- **Phase 6**: Deploy to production with monitoring and support systems

## Consequences

### Positive

- **Enhanced Trust in Voting**: Users can verify that their votes are counted correctly
- **Voting Transparency**: The voting process becomes fully transparent and auditable
- **Vote Tamper Resistance**: Votes cannot be altered once recorded on the blockchain
- **Decentralized Voting Authority**: Reduced reliance on central authorities for trust in the voting process
- **Voter Privacy**: Blockchain can provide anonymity while ensuring vote integrity
- **Voting Resilience**: Voting system can continue to operate even if some nodes fail
- **Focused Resource Allocation**: By using blockchain only where most valuable (voting), we optimize resource usage

### Negative

- **Limited Scope**: Blockchain benefits are limited to the voting system only
- **System Complexity**: Managing two different technologies (blockchain for voting, traditional databases for audit logs)
- **Performance Overhead for Voting**: Blockchain consensus mechanisms add latency to the voting process
- **Development Specialization**: Team will need expertise in both blockchain and traditional database security
- **Integration Challenges**: Ensuring seamless integration between blockchain voting and traditional audit logging

### Neutral

- **Hybrid Technology Approach**: Using the right technology for each component based on requirements
- **Governance Structure**: Will need to establish governance for the blockchain voting network
- **Upgrade Paths**: Need separate upgrade strategies for blockchain and traditional components
- **Security Balance**: Different security models for voting (decentralized) vs. audit logging (centralized with controls)
- **Cost Distribution**: Higher costs for voting system, potentially lower costs for audit logging

## Compliance

- **Voting System Security Audits**: Regular security audits of the blockchain voting system by third-party specialists
- **Smart Contract Verification**: Formal verification of voting-related smart contracts
- **Database Security Audits**: Regular security audits of the traditional database systems used for audit logging
- **Penetration Testing**: Regular penetration testing of both blockchain and traditional components
- **Open Source Voting**: Core blockchain voting components will be open source for community review
- **Comprehensive Monitoring**: Implement monitoring for both blockchain and traditional systems
- **Disaster Recovery**: Establish robust backup and recovery procedures for all system components
- **Documentation**: Maintain detailed documentation of both the blockchain voting architecture and traditional audit logging systems

## Notes

- [ADR 0008](0008-relational-vote-ledger.md) defines the current MVP proposal.
- If a permissioned ledger becomes a firm requirement, Hyperledger Iroha 2 is
  the first open-source Rust-native candidate to evaluate. Fabric remains an
  alternative only if the project accepts a non-Rust adapter and chaincode
  implementation.

- Reference: [Hyperledger Fabric Documentation](https://hyperledger-fabric.readthedocs.io/)
- Reference: [NIST Blockchain Technology Overview](https://nvlpubs.nist.gov/nistpubs/ir/2018/NIST.IR.8202.pdf)
- Reference: [Follow My Vote - Blockchain Voting](https://followmyvote.com/)
- Reference: [Decentralized Identity Foundation](https://identity.foundation/)
- Reference: [Blockchain for Government and Public Services](https://www.oecd.org/gov/innovative-government/blockchain-for-government-and-public-services.htm)
- Reference: [NIST Database Security Guide](https://nvlpubs.nist.gov/nistpubs/SpecialPublications/NIST.SP.800-123.pdf)
- Reference: [OWASP Logging Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Logging_Cheat_Sheet.html)
- Tools: [Hyperledger Fabric](https://www.hyperledger.org/use/fabric), [PostgreSQL](https://www.postgresql.org/)
