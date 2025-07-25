# PowerTo

PowerTo is a civic software platform that allows citizens to report issues in their streets, neighborhoods, cities, or states — such as potholes, public lighting problems, garbage accumulation, or accessibility concerns.

The voting process starts with the citizens directly affected — those who share the street, neighborhood, or district where the issue was reported. This local-first prioritization makes the voting more accurate and avoids distortions. As more people in the area support the request, the issue gains relevance on the platform and may attract broader attention if it has a wider impact.

Once an issue reaches a minimum vote threshold, it moves to the budgeting phase. Pre-approved and verified service providers submit proposals impartially. The system automatically selects the best proposal based on cost, delivery time, and historical performance.

After budget approval by the responsible government entity, the selected provider carries out the service. Upon completion, citizens evaluate the quality of the work and rate the service provider. If the service is deemed unsatisfactory, the company is given a chance to correct or redo the work. The evaluation cycle continues until the problem is satisfactorily resolved.

This system ensures transparency, active citizen participation, and efficiency in public resource management — strengthening trust between the population, service providers, and government.

## Features

- **Issue Submission**: Citizens can post issues affecting their community
- **Democratic Voting**: Community members vote on submitted issues
- **Priority Ranking**: Issues are automatically ranked based on vote counts
- **Cost Analysis**: Most voted issues undergo detailed cost analysis
- **Transparent Process**: Full visibility into the decision-making process
- **Community Collaboration**: Foster cooperation among citizens to solve local problems

## Getting Started

### Prerequisites

_[To be added as the project develops]_

### Installation

_[To be added as the project develops]_

## Usage

### Proposing Issues

To propose a new issue on the PowerTo platform, please follow our [Issue Proposal Model](docs/models/issue-proposal-model.md). This model provides a structured format for submitting issues, ensuring all necessary information is included for proper community evaluation and voting.

_[Additional usage information to be added as the project develops]_

## Technology Stack

_[To be added as the project develops]_

## Architecture

PowerTo follows a set of architectural principles documented in our Architecture Decision Records (ADRs):

- [Semantic Versioning and Conventional Commits](docs/architecture/decisions/0001-semantic-versioning-and-conventional-commits.md)
- [Blockchain for Voting](docs/architecture/decisions/0002-blockchain-for-voting.md)
- [Open-Source Observability Tools](docs/architecture/decisions/0003-opensource-observability-tools.md)

These documents outline our approach to key architectural decisions that shape the PowerTo platform.

### Automated Releases

PowerTo uses GitHub Actions to automatically create releases when changes are merged to the main branch. The release process:

1. Analyzes commits since the last release using conventional commit messages
2. Determines the next version number based on semantic versioning rules
3. Generates comprehensive release notes
4. Creates a new tag and GitHub release
5. Updates the CHANGELOG.md file

For more details, see the [GitHub Configuration documentation](.github/README.md#automatic-release-workflow).

## Documentation

Comprehensive documentation for PowerTo is available in our [documentation site](docs/website/README.md). The documentation is built using [Docusaurus](https://docusaurus.io/) and includes:

- Introduction to PowerTo
- Architecture Decision Records
- Development Guides
- Data Models

To run the documentation site locally:

```bash
# Install dependencies
cd docs/website
npm install

# Start the development server
npm start
```

This will start a local development server and open up a browser window. Most changes are reflected live without having to restart the server.

## Contributing

We welcome contributions from everyone! Please see our [CONTRIBUTING.md](CONTRIBUTING.md) file for details on how to get started. The project uses various code formatting tools (EditorConfig, Prettier, Black, etc.) to maintain consistent code style - see the contributing guide for more information.

## License

This project is licensed under the [MIT License](LICENSE) - see the LICENSE file for details.

## Contact

_[To be added as the project develops]_
