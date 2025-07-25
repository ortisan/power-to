---
id: 0001-semantic-versioning-and-conventional-commits
title: Semantic Versioning and Conventional Commits
---

# Semantic Versioning and Conventional Commits

## Status

PROPOSED

## Context

As the PowerTo platform evolves, we need a consistent and clear approach to versioning our software and structuring our commit messages. This ensures that:

1. Users and developers can understand the impact of new releases
2. Breaking changes are clearly communicated
3. Dependency management is predictable
4. Automated tools can generate changelogs and determine appropriate version bumps
5. Contributors follow a consistent format for commit messages
6. The project history is readable, navigable, and useful

Key questions to address:
- How should we version our software releases?
- What commit message format should contributors follow?
- How can we automate version management and changelog generation?
- How do we ensure compliance with these standards?

## Decision

We will adopt Semantic Versioning (SemVer) for version numbering and Conventional Commits for commit message formatting.

### 1. Semantic Versioning (SemVer)

We will follow the [Semantic Versioning 2.0.0](https://semver.org/) specification:

- Version numbers will follow the format: MAJOR.MINOR.PATCH (e.g., 1.2.3)
- MAJOR version increments for incompatible API changes
- MINOR version increments for backward-compatible functionality additions
- PATCH version increments for backward-compatible bug fixes
- Pre-release versions may be denoted with a hyphen (e.g., 1.0.0-alpha.1)
- Build metadata may be appended with a plus sign (e.g., 1.0.0+20130313144700)

Additional guidelines:
- Initial development (pre-1.0.0) may have rapid changes; version 0.y.z indicates the API is not stable
- Version 1.0.0 defines the public API
- Deprecation of features should be announced before removal in a major version
- Dependencies must specify version ranges that are compatible with our SemVer approach

### 2. Conventional Commits

We will follow the [Conventional Commits 1.0.0](https://www.conventionalcommits.org/) specification:

Commit messages will follow this structure:
```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

Types include:
- `feat`: A new feature (correlates with MINOR in SemVer)
- `fix`: A bug fix (correlates with PATCH in SemVer)
- `docs`: Documentation only changes
- `style`: Changes that do not affect the meaning of the code
- `refactor`: Code changes that neither fix a bug nor add a feature
- `perf`: Changes that improve performance
- `test`: Adding missing tests or correcting existing tests
- `build`: Changes to the build system or external dependencies
- `ci`: Changes to CI configuration files and scripts
- `chore`: Other changes that don't modify src or test files
- `revert`: Reverts a previous commit

Breaking changes must be indicated with a `!` after the type/scope or with a `BREAKING CHANGE:` footer.

Examples:
```
feat: add user voting capability

fix(api): prevent race condition in vote counting

feat!: redesign user interface

feat(api): allow provided config object to extend other configs

BREAKING CHANGE: `extends` key in config file is now used for extending other config files
```

### 3. Implementation and Tooling

We will implement these standards with the following tools:

- Use Git tags for version marking
- Implement commit message linting with `commitlint`
- Use `standard-version` or similar tools for automated version bumping and changelog generation
- Configure CI/CD pipelines to validate commit messages
- Provide commit message templates for contributors

## Consequences

### Positive

- Clear communication of changes to users and developers
- Automated changelog generation
- Simplified dependency management
- More structured and navigable project history
- Easier onboarding for new contributors
- Better integration with semantic release tools

### Negative

- Learning curve for contributors not familiar with these standards
- Slightly more overhead when creating commits
- Potential for rejected commits if they don't follow the convention

### Neutral

- Need for tooling to enforce and leverage these standards
- Regular reviews to ensure compliance

## Compliance

- Commit hooks will validate commit messages against the Conventional Commits format
- CI/CD pipelines will verify commit message compliance
- Pull request templates will include reminders about these standards
- Code review process will include checking for proper commit messages
- Release process will verify that version bumps follow SemVer principles
- Documentation will be provided to help contributors understand and follow these standards

## Notes

- Reference: [Semantic Versioning 2.0.0](https://semver.org/)
- Reference: [Conventional Commits 1.0.0](https://www.conventionalcommits.org/)
- Reference: [Angular Commit Message Guidelines](https://github.com/angular/angular/blob/master/CONTRIBUTING.md#commit)
- Reference: [Keep a Changelog](https://keepachangelog.com/)
- Tools: [commitlint](https://commitlint.js.org/), [standard-version](https://github.com/conventional-changelog/standard-version), [semantic-release](https://semantic-release.gitbook.io/)
