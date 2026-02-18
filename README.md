# bd-complete

Shell completion generator for [beads](https://github.com/steveyegge/beads) (`bd`) — the lightweight, git-native issue tracker.

`bd-complete` introspects the live `bd` binary's help output and generates shell-specific completion scripts that handle commands, subcommands, flags, and aliases.

## Supported Shells

- **Bash** — uses `bash-completion` framework
- **Fish** — uses Fish's built-in `complete` system

## Requirements

- Rust toolchain (for building)
- `bd` must be installed and on your `PATH`
- For Bash: `bash-completion` package installed

## Installation

### From Source

```bash
cargo install --path .
```

### Homebrew (macOS)

```bash
brew tap jtperry/tap
brew install bd-complete
```

### Script Install

```bash
curl -fsSL https://raw.githubusercontent.com/jtperry/bd-complete/main/install.sh | bash
```

## Usage

### Generate and Install Completions

**Bash:**

```bash
# Generate to stdout
bd-complete generate --shell bash

# Write to bash-completion directory
bd-complete generate --shell bash --output /usr/local/etc/bash_completion.d/bd

# Or for Linux
bd-complete generate --shell bash --output ~/.local/share/bash-completion/completions/bd
```

**Fish:**

```bash
# Generate to stdout
bd-complete generate --shell fish

# Write to Fish completions directory
bd-complete generate --shell fish --output ~/.config/fish/completions/bd.fish
```

### Options

```
Usage: bd-complete generate --shell <SHELL> [--output <FILE>]

Commands:
  generate    Generate a shell completion script

Options:
  --shell <SHELL>    Shell type: bash, fish
  --output <FILE>    Write to file instead of stdout (alias: -o)
  --help             Show help
```

## How It Works

1. Runs `bd --help` and parses the Cobra-style output
2. Recursively runs `bd <subcommand> --help` for each subcommand (up to 2 levels deep)
3. Builds an internal command tree of commands, subcommands, flags, and aliases
4. Generates a shell-specific completion script from the tree

The generated scripts handle:
- Top-level and nested subcommand completion
- Command aliases (e.g., `create`/`new`)
- Flag name completion (long and short forms)
- Flag value completion (file paths for `--db`, etc.)

## Development

```bash
# Build
cargo build

# Run tests (29 tests covering parser, bash, and fish generators)
cargo test

# Generate bash completion for testing
cargo run -- generate --shell bash
```

Zero external dependencies — pure Rust standard library.

## Project Structure

```
src/
├── main.rs           # CLI entry point and argument parsing
├── command_tree.rs   # Data model (Command, Flag, CommandTree)
├── parser.rs         # Parses bd --help output into command tree
├── bash.rs           # Bash completion script generator
└── fish.rs           # Fish completion script generator
```

## License

Apache License 2.0 — see [LICENSE](LICENSE) for details.

Copyright 2006 JT Perry
