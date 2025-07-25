# Code Formatter Setup Guide

This guide provides instructions for setting up and using the code formatters configured for the PowerTo project.

## Overview

PowerTo uses several code formatting tools to maintain consistent code style across the project:

- **EditorConfig**: Basic file formatting for all files
- **Prettier**: For JavaScript, TypeScript, CSS, HTML, and other frontend files
- **Black**: For Python files
- **isort**: For Python import sorting
- **golangci-lint**: For Go files
- **clang-format**: For C++ files

## EditorConfig

EditorConfig helps maintain consistent coding styles across different editors and IDEs.

### Installation

Most modern editors and IDEs support EditorConfig natively or through plugins:

- **VS Code**: Install the "EditorConfig for VS Code" extension
- **IntelliJ IDEA/WebStorm/PyCharm**: Supported natively
- **Sublime Text**: Install the "EditorConfig" package
- **Vim**: Install the "editorconfig-vim" plugin
- **Emacs**: Install the "editorconfig-emacs" package

No additional configuration is needed as the project already includes an `.editorconfig` file.

## Prettier

Prettier is an opinionated code formatter for JavaScript, TypeScript, CSS, HTML, and more.

### Installation

1. Install Node.js and npm if you haven't already
2. Install Prettier globally:
   ```
   npm install -g prettier
   ```
3. Install editor plugins (recommended):
   - **VS Code**: "Prettier - Code formatter" extension
   - **IntelliJ IDEA/WebStorm**: "Prettier" plugin
   - **Sublime Text**: "JsPrettier" package

### Usage

- **Command line**: `prettier --write "**/*.{js,jsx,ts,tsx,json,css,scss,html}"`
- **VS Code**: Enable "Format On Save" and set Prettier as the default formatter
- **IntelliJ/WebStorm**: Enable "Prettier" in Settings > Languages & Frameworks > JavaScript > Prettier

## Black (Python)

Black is an uncompromising Python code formatter.

### Installation

1. Install Black:
   ```
   pip install black
   ```
2. Install editor plugins (recommended):
   - **VS Code**: "Python" extension includes Black support
   - **PyCharm**: "BlackConnect" plugin
   - **Vim**: Configure with ALE or similar

### Usage

- **Command line**: `black .`
- **VS Code**: Enable format on save with the Python extension
- **PyCharm**: Configure external tool or use BlackConnect

## isort (Python)

isort automatically sorts Python imports.

### Installation

1. Install isort:
   ```
   pip install isort
   ```
2. Editor integration:
   - **VS Code**: "Python" extension includes isort support
   - **PyCharm**: Configure as external tool

### Usage

- **Command line**: `isort .`
- **VS Code**: Configure Python extension to run isort

## golangci-lint (Go)

golangci-lint is a fast Go linters runner that includes formatting checks.

### Installation

1. Install golangci-lint:
   ```
   # macOS
   brew install golangci-lint
   
   # Windows
   scoop install golangci-lint
   
   # Linux
   curl -sSfL https://raw.githubusercontent.com/golangci/golangci-lint/master/install.sh | sh -s -- -b $(go env GOPATH)/bin
   ```
2. Editor integration:
   - **VS Code**: "Go" extension includes support
   - **GoLand**: Native support
   - **Vim/Neovim**: Configure with ALE or similar

### Usage

- **Command line**: `golangci-lint run`
- **VS Code**: Configure Go extension to use golangci-lint

## clang-format (C++)

clang-format is a tool to format C/C++/Java/JavaScript/Objective-C/Protobuf/C# code.

### Installation

1. Install clang-format:
   ```
   # macOS
   brew install clang-format
   
   # Ubuntu/Debian
   apt-get install clang-format
   
   # Windows
   # Install via LLVM installer or Visual Studio
   ```
2. Editor integration:
   - **VS Code**: "C/C++" extension includes support
   - **CLion**: Native support
   - **Visual Studio**: "ClangFormat" extension

### Usage

- **Command line**: `clang-format -i file.cpp`
- **VS Code**: Configure C/C++ extension to use clang-format
- **CLion/Visual Studio**: Enable format on save

## Pre-commit Hooks (Optional)

For automated formatting on commit, you can set up pre-commit hooks:

1. Install pre-commit:
   ```
   pip install pre-commit
   ```

2. Create a `.pre-commit-config.yaml` file in the project root with the following content:
   ```yaml
   repos:
   - repo: https://github.com/pre-commit/pre-commit-hooks
     rev: v4.4.0
     hooks:
     - id: trailing-whitespace
     - id: end-of-file-fixer
     - id: check-yaml
     - id: check-added-large-files
   
   - repo: https://github.com/pycqa/isort
     rev: 5.12.0
     hooks:
     - id: isort
   
   - repo: https://github.com/psf/black
     rev: 23.3.0
     hooks:
     - id: black
   
   - repo: https://github.com/pre-commit/mirrors-prettier
     rev: v3.0.0
     hooks:
     - id: prettier
   ```

3. Install the hooks:
   ```
   pre-commit install
   ```

## Troubleshooting

If you encounter issues with any of the formatters:

1. Ensure you have the latest version of the tool installed
2. Check that your editor plugin is properly configured
3. Verify that the tool's configuration file is in the correct location
4. Try running the formatter from the command line to see detailed error messages

For more help, consult the documentation for the specific tool or reach out to the project maintainers.
