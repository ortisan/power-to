---
id: intro
title: Introduction to PowerTo
sidebar_label: Introduction
slug: /
---

# Welcome to PowerTo Documentation

PowerTo is an open-source civic platform intended to help residents identify,
prioritize, and follow local problems through a transparent process. The
long-term product includes issue reporting, moderation, territorially scoped
participation, delivery tracking, and public evaluation.

> **Current status:** architecture and executable backend foundation. The Rust
> workspace, health API, Atlas/PostGIS bootstrap, and local OpenTelemetry with
> Victoria backends exist. The civic workflows and web, Android, and iOS
> clients do not. PowerTo is not ready for real voting or procurement.

## About This Documentation

This documentation is designed to help you understand and use the PowerTo platform. Whether you're a user, contributor, or developer, you'll find information to help you get the most out of PowerTo.

## Intended Product Capabilities

- **Issue Submission**: Citizens can post issues affecting their community
- **Democratic Voting**: Community members vote on submitted issues
- **Priority Ranking**: Issues are automatically ranked based on vote counts
- **Cost Analysis**: Most voted issues undergo detailed cost analysis
- **Transparent Process**: Full visibility into the decision-making process
- **Community Collaboration**: Foster cooperation among citizens to solve local problems

The deliberately smaller first release and its exclusions are documented in
the [MVP scope](product/mvp-scope).

## Documentation Sections

- **[Product Scope](product/mvp-scope)**: Understand the long-term product and the recommended first release
- **[Architecture](architecture/overview)**: Learn how PowerTo is structured and why
- **[Technology Stack](architecture/technology-stack)**: Review accepted and proposed technology choices
- **[User Media Storage](architecture/media-storage)**: Review safe R2, S3, and GCS upload and processing flows
- **[Mobile and Road Sensing](architecture/mobile-sensing)**: Review Android/iOS capture, geofencing, motion evidence, safety, and privacy
- **[Architecture Decisions](architecture/decisions/0000-adr-template)**: Review individual decisions and their status
- **[Development](development/formatter-setup)**: Find guides for developers contributing to PowerTo
- **[Models](models/issue-proposal-model)**: Understand the data models used in PowerTo

## Getting Started

To get started with PowerTo, check out the following resources:

- [Issue Proposal Model](models/issue-proposal-model): Learn how to propose issues on the platform
- [Architecture Overview](architecture/overview): Understand the proposed system boundaries
- [Technology Stack](architecture/technology-stack): Review Rust, Diesel, Atlas, OpenTelemetry, and the supporting stack
- [User Media Storage](architecture/media-storage): Understand portable photo and video handling
- [Mobile and Road Sensing](architecture/mobile-sensing): Understand mobile issue capture and confidence-scored road observations
- [Contributing Guide](https://github.com/ortisan/power-to/blob/main/CONTRIBUTING.md): Find out how to contribute to the PowerTo project
- [Backend Guide](https://github.com/ortisan/power-to/blob/main/backend/README.md): Run and validate the Rust foundation
- [GitHub Repository](https://github.com/ortisan/power-to): Access the source code and contribute to development

## Contributing to Documentation

We welcome contributions to improve this documentation. If you find any errors or have suggestions for improvements, please submit an issue or pull request on our [GitHub repository](https://github.com/ortisan/power-to).
