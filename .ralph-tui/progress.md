# bd-complete Progress

## Codebase Patterns

- **Cobra help format**: Section headers are lines ending with `:` that don't start with whitespace. Commands are indented with 2+ spaces separating name from description. Flags use `-short, --long type  description` format.
- **Top-level vs subcommand help**: At root level, cobra puts all flags under `Flags:`. Subcommands split into `Flags:` (local) and `Global Flags:` (inherited).
- **Value type detection**: Cobra flag value types appear as a single lowercase word after the `--flag` name (e.g., `--db string`). Boolean flags have no value type token.
- **Shellcheck compliance**: Use `mapfile -t COMPREPLY < <(compgen ...)` instead of `COMPREPLY=( $(compgen ...) )` to avoid SC2207. Bash case patterns with spaces must be quoted (e.g., `"dep add"`).

---

## 2026-02-18 - US-001
- **What was implemented**: Complete cobra-style help output parser that builds a structured `CommandTree` from `bd --help` output. Includes recursive parsing of subcommand help for 2 levels deep.
- **Files changed**:
  - `Cargo.toml` — New Rust project (bd-complete, edition 2021)
  - `src/main.rs` — Entry point, wires up modules, runs `build_command_tree("bd")`
  - `src/command_tree.rs` — Data structures: `CommandTree`, `Command`, `Flag`, `CommandGroup`
  - `src/parser.rs` — Parser logic: `parse_help_output()`, `parse_flag_line()`, `parse_command_line()`, `split_flag_description()`, `run_help()`, `build_command_tree()` + 10 unit tests
- **Learnings:**
  - Cobra help output uses double-space (2+) as delimiter between command name and description. Single spaces within command names (like `close-eligible`) don't trigger splitting.
  - Flag description boundary detection requires walking tokens and checking for 2+ space gaps, with special handling for value type tokens that appear between the flag name and description.
  - `bd` has 88 top-level commands across 9 groups, with subcommands up to 2 levels deep. Most commands have local flags discovered via recursive `--help`.
  - Default values in cobra are formatted as `(default "value")` or `(default: value)` at the end of flag descriptions.
---

## 2026-02-18 - US-002
- **What was implemented**: Bash completion script generator that produces a shellcheck-clean completion script from the parsed CommandTree. Added CLI interface (`bd-complete generate --shell bash [--output FILE]`).
- **Files changed**:
  - `src/bash.rs` — New module: `generate_bash_completion()` produces a complete Bash completion script with Apache 2.0 header, command tree walking, subcommand/flag/flag-value completion, and alias support. 9 unit tests including shellcheck validation.
  - `src/main.rs` — Rewritten with CLI argument parsing: `generate --shell bash [--output FILE]` command, supports stdout and file output.
- **Learnings:**
  - Bash `case` patterns cannot contain unquoted spaces. Multi-word patterns like `dep add` must be quoted as `"dep add"`.
  - Shellcheck SC2207 warns against `COMPREPLY=( $(compgen ...) )`. The compliant form is `mapfile -t COMPREPLY < <(compgen ...)`.
  - The completion script uses `_init_completion` from bash-completion, which provides `cur`, `prev`, `words`, `cword` variables.
  - For flag value completion, file-like flags (containing "file", "path", or named "db") use `compgen -f` for filesystem completion; other typed flags return empty COMPREPLY.
  - Aliases are handled by generating alternate case patterns (e.g., `create|new)`).
---
