# Contributing to PowerTo

Thank you for considering contributing to PowerTo! This document outlines the process for contributing to the project and provides guidelines to help you get started.

## Code of Conduct

By participating in this project, you agree to abide by our [Code of Conduct](CODE_OF_CONDUCT.md). Please read it before contributing.

## How Can I Contribute?

### Reporting Bugs

This section guides you through submitting a bug report for PowerTo. Following these guidelines helps maintainers understand your report, reproduce the issue, and find related reports.

Before creating bug reports, please check the existing issues as you might find that the bug has already been reported. When you are creating a bug report, please include as many details as possible:

* **Use a clear and descriptive title** for the issue to identify the problem.
* **Describe the exact steps which reproduce the problem** in as much detail as possible.
* **Provide specific examples to demonstrate the steps**. Include links to files or GitHub projects, or copy/pasteable snippets, which you use in those examples.
* **Describe the behavior you observed after following the steps** and point out what exactly is the problem with that behavior.
* **Explain which behavior you expected to see instead and why.**
* **Include screenshots and animated GIFs** which show you following the described steps and clearly demonstrate the problem.
* **If the problem wasn't triggered by a specific action**, describe what you were doing before the problem happened.

### Suggesting Enhancements

This section guides you through submitting an enhancement suggestion for PowerTo, including completely new features and minor improvements to existing functionality.

* **Use a clear and descriptive title** for the issue to identify the suggestion.
* **Provide a step-by-step description of the suggested enhancement** in as much detail as possible.
* **Provide specific examples to demonstrate the steps**. Include copy/pasteable snippets which you use in those examples.
* **Describe the current behavior** and **explain which behavior you expected to see instead** and why.
* **Include screenshots and animated GIFs** which help you demonstrate the steps or point out the part of PowerTo which the suggestion is related to.
* **Explain why this enhancement would be useful** to most PowerTo users.
* **List some other applications where this enhancement exists.**

### Pull Requests

* Fill in the required template
* Do not include issue numbers in the PR title
* Include screenshots and animated GIFs in your pull request whenever possible
* Follow the style guides
* Document new code
* End all files with a newline
* Avoid platform-dependent code

## Development Setup

To set up PowerTo for local development:

1. Fork the PowerTo repository on GitHub.
2. Clone your fork locally:
   ```
   git clone https://github.com/your-username/power-to.git
   cd power-to
   ```
3. Create a branch for local development:
   ```
   git checkout -b name-of-your-bugfix-or-feature
   ```
   Now you can make your changes locally.

4. When you're done making changes, check that your changes pass the tests and lint checks:
   ```
   # Commands will be added as the project develops
   ```

5. Commit your changes and push your branch to GitHub:
   ```
   git add .
   git commit -m "Your detailed description of your changes."
   git push origin name-of-your-bugfix-or-feature
   ```

6. Submit a pull request through the GitHub website.

## Coding Standards

### Git Commit Messages

We follow the [Conventional Commits](https://www.conventionalcommits.org/) specification for commit messages. This provides a structured format that makes the project history more readable and enables automated tools to generate changelogs and determine version bumps.

Commit messages must follow this structure:
```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

Types include:
* `feat`: A new feature (correlates with MINOR in SemVer)
* `fix`: A bug fix (correlates with PATCH in SemVer)
* `docs`: Documentation only changes
* `style`: Changes that do not affect the meaning of the code
* `refactor`: Code changes that neither fix a bug nor add a feature
* `perf`: Changes that improve performance
* `test`: Adding missing tests or correcting existing tests
* `build`: Changes to the build system or external dependencies
* `ci`: Changes to CI configuration files and scripts
* `chore`: Other changes that don't modify src or test files
* `revert`: Reverts a previous commit

Breaking changes must be indicated with a `!` after the type/scope or with a `BREAKING CHANGE:` footer.

Examples:
```
feat: add user voting capability
fix(api): prevent race condition in vote counting
feat!: redesign user interface
```

#### Commit Message Enforcement

We use [commitlint](https://commitlint.js.org/) to enforce conventional commit messages. This is set up as a pre-commit hook, so commits that don't follow the convention will be rejected.

To help you write conventional commits, we provide two options:

##### Option 1: Commit Message Template

We provide a commit message template that you can use as a guide:

1. Configure Git to use the template:
   ```
   git config --local commit.template .gitmessage
   ```

2. When you commit, Git will open your editor with the template:
   ```
   git commit
   ```

3. Follow the template to write your commit message.

##### Option 2: Interactive Commit Tool (Commitizen)

We also provide [Commitizen](http://commitizen.github.io/cz-cli/), which is an interactive command-line tool that guides you through creating a conventional commit message:

1. Install the dependencies:
   ```
   npm install
   ```

2. Use the commit script instead of `git commit`:
   ```
   npm run commit
   ```

3. Follow the prompts to create your commit message.

General guidelines:
* Use the present tense ("Add feature" not "Added feature")
* Use the imperative mood ("Move cursor to..." not "Moves cursor to...")
* Limit the first line to 72 characters or less
* Reference issues and pull requests in the body or footer

### Code Style

We use automated formatting tools to maintain consistent code style across the project. The following tools are configured:

* **EditorConfig**: Basic file formatting for all files. See [.editorconfig](.editorconfig)
* **Prettier**: For JavaScript, TypeScript, CSS, HTML, and other frontend files. See [.prettierrc.json](.prettierrc.json)
* **Black**: For Python files. See [pyproject.toml](pyproject.toml)
* **isort**: For Python import sorting. See [pyproject.toml](pyproject.toml)
* **golangci-lint**: For Go files. See [.golangci.yml](.golangci.yml)
* **clang-format**: For C++ files. See [.clang-format](.clang-format)

Please ensure you have these tools installed and configured in your development environment. Many editors and IDEs have plugins that support these tools and can format code automatically on save.

We also provide a pre-commit hooks configuration (`.pre-commit-config.yaml`) that automatically formats code when you commit changes and validates commit messages. To use it:

1. Install pre-commit: `pip install pre-commit`
2. Set up the hooks: 
   ```
   pre-commit install
   pre-commit install --hook-type commit-msg
   ```

   The first command installs hooks that run before the commit is created, while the second installs hooks that validate the commit message.

For detailed setup instructions for all formatting tools, see our [Formatter Setup Guide](docs/development/formatter-setup.md).

#### General Guidelines

* Follow the formatting rules defined in the configuration files
* Use meaningful variable and function names
* Write clear comments for complex logic
* Keep functions small and focused on a single responsibility
* Write unit tests for your code

## License

By contributing, you agree that your contributions will be licensed under the project's [MIT License](LICENSE).

## Questions?

If you have any questions, please feel free to contact the project maintainers.
