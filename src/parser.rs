// Copyright 2006 JT Perry
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::command_tree::{Command, CommandGroup, CommandTree, Flag};
use std::collections::BTreeMap;
use std::io;
use std::process;

/// Sections we recognize in cobra-style help output.
#[derive(Debug, PartialEq)]
enum Section {
    Preamble,
    Usage,
    Aliases,
    Commands(String), // group name like "Available Commands", "Working With Issues", etc.
    Flags,
    GlobalFlags,
}

/// Parse the help output of a cobra-style CLI command.
pub fn parse_help_output(text: &str) -> (Command, Vec<Flag>, Vec<CommandGroup>) {
    let mut description_lines: Vec<&str> = Vec::new();
    let mut usage: Option<String> = None;
    let mut aliases: Vec<String> = Vec::new();
    let mut flags: Vec<Flag> = Vec::new();
    let mut global_flags: Vec<Flag> = Vec::new();
    let mut subcommands: BTreeMap<String, Command> = BTreeMap::new();
    let mut groups: Vec<CommandGroup> = Vec::new();
    let mut current_group_name: Option<String> = None;
    let mut current_group_cmds: Vec<String> = Vec::new();

    let mut section = Section::Preamble;

    for line in text.lines() {
        // Detect section headers (lines ending with ':' and not starting with whitespace)
        if !line.starts_with(' ') && !line.starts_with('\t') && line.ends_with(':') {
            // Flush current group if any
            if let Some(gname) = current_group_name.take() {
                if !current_group_cmds.is_empty() {
                    groups.push(CommandGroup {
                        name: gname,
                        commands: std::mem::take(&mut current_group_cmds),
                    });
                }
            }

            let header = line.trim_end_matches(':').trim();
            section = match header {
                "Usage" => Section::Usage,
                "Aliases" => Section::Aliases,
                "Flags" => Section::Flags,
                "Global Flags" => Section::GlobalFlags,
                _ => {
                    // Any other header is a command group
                    current_group_name = Some(header.to_string());
                    Section::Commands(header.to_string())
                }
            };
            continue;
        }

        // Skip blank lines in some contexts
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        match &section {
            Section::Preamble => {
                // Lines before the first section header are description
                if !trimmed.starts_with("Use \"") {
                    description_lines.push(trimmed);
                }
            }
            Section::Usage => {
                if usage.is_none() {
                    usage = Some(trimmed.to_string());
                }
            }
            Section::Aliases => {
                // "create, new" — split on commas
                for alias in trimmed.split(',') {
                    let a = alias.trim().to_string();
                    if !a.is_empty() {
                        aliases.push(a);
                    }
                }
            }
            Section::Commands(group_name) => {
                if let Some(cmd) = parse_command_line(trimmed) {
                    let mut c = cmd;
                    c.group = Some(group_name.clone());
                    current_group_cmds.push(c.name.clone());
                    subcommands.insert(c.name.clone(), c);
                }
            }
            Section::Flags => {
                if let Some(flag) = parse_flag_line(trimmed) {
                    flags.push(flag);
                }
            }
            Section::GlobalFlags => {
                if let Some(flag) = parse_flag_line(trimmed) {
                    global_flags.push(flag);
                }
            }
        }
    }

    // Flush final group
    if let Some(gname) = current_group_name.take() {
        if !current_group_cmds.is_empty() {
            groups.push(CommandGroup {
                name: gname,
                commands: current_group_cmds,
            });
        }
    }

    let description = description_lines.join(" ");

    // The first alias is usually the command name itself in cobra
    // e.g. "create, new" — "create" is the name, "new" is the alias
    // We store only the extras as aliases
    if aliases.len() > 1 {
        aliases.remove(0); // remove the command's own name
    } else {
        aliases.clear();
    }

    let mut cmd = Command::new("", description);
    cmd.usage = usage;
    cmd.aliases = aliases;
    cmd.flags = flags;
    cmd.subcommands = subcommands;

    (cmd, global_flags, groups)
}

/// Parse a command line like "  create           Create a new issue..."
fn parse_command_line(line: &str) -> Option<Command> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Split on first run of 2+ spaces (command name is followed by spacing then description)
    let parts: Vec<&str> = trimmed.splitn(2, "  ").collect();
    let name = parts[0].trim();
    let description = if parts.len() > 1 {
        parts[1].trim()
    } else {
        ""
    };

    if name.is_empty() || name.starts_with('-') {
        return None;
    }

    Some(Command::new(name, description))
}

/// Parse a flag line like "  -v, --verbose   Enable verbose output"
/// or "      --db string   Database path (default: auto-discover)"
fn parse_flag_line(line: &str) -> Option<Flag> {
    let trimmed = line.trim();
    if trimmed.is_empty() || !trimmed.contains('-') {
        return None;
    }

    let mut short: Option<char> = None;
    let mut long = String::new();
    let mut value_type: Option<String> = None;
    let mut default: Option<String> = None;

    // Split into flag part and description part.
    let (flag_part, desc_part) = split_flag_description(trimmed);

    // Parse flag tokens from flag_part
    let tokens: Vec<&str> = flag_part.split_whitespace().collect();
    let mut i = 0;
    while i < tokens.len() {
        let tok = tokens[i].trim_end_matches(',');
        if tok.starts_with("--") {
            long = tok.trim_start_matches('-').to_string();
        } else if tok.starts_with('-') && tok.len() == 2 {
            short = tok.chars().nth(1);
        } else if !tok.is_empty() && !tok.starts_with('-') {
            value_type = Some(tok.to_string());
        }
        i += 1;
    }

    if long.is_empty() {
        return None;
    }

    let description = desc_part.to_string();

    // Extract default value from description like (default: "value") or (default "value")
    if let Some(start) = description.find("(default") {
        if let Some(end) = description[start..].find(')') {
            let default_str = &description[start..start + end + 1];
            // Extract the value after "default" or "default:"
            let val = default_str
                .trim_start_matches("(default")
                .trim_start_matches(':')
                .trim()
                .trim_end_matches(')')
                .trim()
                .trim_matches('"')
                .to_string();
            if !val.is_empty() {
                default = Some(val);
            }
        }
    }

    Some(Flag {
        long,
        short,
        description,
        value_type,
        default,
    })
}

/// Split a flag line into the flag portion and description portion.
/// The description starts after the first run of 2+ spaces that follows a flag token.
fn split_flag_description(line: &str) -> (&str, &str) {
    // Find pattern: non-space chars, then 2+ spaces — that's where description starts
    let bytes = line.as_bytes();
    let mut i = 0;
    let len = bytes.len();

    // Skip leading whitespace
    while i < len && bytes[i] == b' ' {
        i += 1;
    }

    // We need to find the boundary: after the flag tokens (which may include a value type),
    // there will be 2+ spaces followed by the description text.
    // Walk through tokens separated by spaces. After each token, check if next gap is 2+ spaces.
    let mut found_flag = false;
    while i < len {
        // Skip a token
        let token_start = i;
        while i < len && bytes[i] != b' ' {
            i += 1;
        }
        let token = &line[token_start..i];

        if token.contains('-') {
            found_flag = true;
        }

        // Count spaces
        let space_start = i;
        while i < len && bytes[i] == b' ' {
            i += 1;
        }
        let spaces = i - space_start;

        // If we've seen a flag and there's a gap of 2+, this is the boundary
        // But we need to handle value types (single words like "string" right after the flag)
        if found_flag && spaces >= 2 && i < len {
            // Check if next token looks like a value type (lowercase word, no dashes)
            let next_start = i;
            let mut j = i;
            while j < len && bytes[j] != b' ' {
                j += 1;
            }
            let next_token = &line[next_start..j];

            // Value types are: string, strings, int, duration, etc. (lowercase, no dashes, short)
            if is_value_type(next_token) {
                // Include value type in flag part, continue
                i = j;
                // Skip spaces after value type
                while i < len && bytes[i] == b' ' {
                    i += 1;
                }
                return (&line[..j], &line[i..]);
            }

            return (&line[..space_start], &line[i..]);
        }
    }

    (line, "")
}

fn is_value_type(s: &str) -> bool {
    matches!(
        s,
        "string" | "strings" | "int" | "int32" | "int64" | "float" | "float64" | "duration" | "uint" | "count"
    )
}

/// Run a command and capture its help output.
pub fn run_help(command: &[&str]) -> io::Result<String> {
    let output = process::Command::new(command[0])
        .args(&command[1..])
        .arg("--help")
        .output()?;

    // Cobra outputs help to stdout on success, stderr on error
    let text = if output.stdout.is_empty() {
        String::from_utf8_lossy(&output.stderr).into_owned()
    } else {
        String::from_utf8_lossy(&output.stdout).into_owned()
    };
    Ok(text)
}

/// Build a full CommandTree by running `bd --help` and recursively parsing subcommands.
pub fn build_command_tree(binary: &str) -> io::Result<CommandTree> {
    let help_text = run_help(&[binary])?;
    let (mut root_cmd, global_flags, groups) = parse_help_output(&help_text);
    root_cmd.name = binary.to_string();

    // Recursively parse each subcommand
    let subcommand_names: Vec<String> = root_cmd.subcommands.keys().cloned().collect();
    for name in subcommand_names {
        if let Ok(sub_help) = run_help(&[binary, &name]) {
            let (parsed, _sub_globals, _sub_groups) = parse_help_output(&sub_help);
            let entry = root_cmd.subcommands.get_mut(&name).unwrap();
            entry.flags = parsed.flags;
            entry.aliases = parsed.aliases;
            entry.usage = parsed.usage;

            // If this subcommand itself has subcommands, recurse one more level
            if !parsed.subcommands.is_empty() {
                for (sub_name, mut sub_cmd) in parsed.subcommands {
                    if let Ok(sub_sub_help) = run_help(&[binary, &name, &sub_name]) {
                        let (parsed2, _, _) = parse_help_output(&sub_sub_help);
                        sub_cmd.flags = parsed2.flags;
                        sub_cmd.aliases = parsed2.aliases;
                        sub_cmd.usage = parsed2.usage;
                        // Could recurse deeper, but 2 levels covers bd's structure
                        sub_cmd.subcommands = parsed2.subcommands;
                    }
                    entry.subcommands.insert(sub_name, sub_cmd);
                }
            }
        }
    }

    let mut tree = CommandTree::new(root_cmd);
    // At the root level, cobra puts global flags under "Flags:" (no separate "Global Flags:").
    // Use the root flags as global flags if no explicit Global Flags section was found.
    tree.global_flags = if global_flags.is_empty() {
        tree.root.flags.clone()
    } else {
        global_flags
    };
    tree.groups = groups;
    Ok(tree)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BD_HELP: &str = r#"Issues chained together like beads. A lightweight issue tracker with first-class dependency support.

Usage:
  bd [flags]
  bd [command]

Working With Issues:
  children         List child beads of a parent
  close            Close one or more issues
  create           Create a new issue (or multiple issues from markdown file)
  delete           Delete one or more issues and clean up references

Views & Reports:
  count            Count issues matching filters
  status           Show issue database overview and statistics

Additional Commands:
  help             Help about any command
  version          Print version information

Flags:
      --db string                 Database path (default: auto-discover .beads/*.db)
  -h, --help                      help for bd
      --json                      Output in JSON format
  -q, --quiet                     Suppress non-essential output (errors only)
  -v, --verbose                   Enable verbose/debug output

Use "bd [command] --help" for more information about a command."#;

    const CREATE_HELP: &str = r#"Create a new issue (or multiple issues from markdown file)

Usage:
  bd create [title] [flags]

Aliases:
  create, new

Flags:
      --acceptance string       Acceptance criteria
  -a, --assignee string         Assignee
  -d, --description string      Issue description
  -f, --file string             Create multiple issues from markdown file
  -h, --help                    help for create
  -l, --labels strings          Labels (comma-separated)
  -p, --priority string         Priority (0-4 or P0-P4, 0=highest) (default "2")
  -t, --type string             Issue type (default "task")

Global Flags:
      --db string                 Database path (default: auto-discover .beads/*.db)
      --json                      Output in JSON format
  -v, --verbose                   Enable verbose/debug output"#;

    const EPIC_HELP: &str = r#"Epic management commands

Usage:
  bd epic [command]

Available Commands:
  close-eligible  Close epics where all children are complete
  status          Show epic completion status

Flags:
  -h, --help   help for epic

Global Flags:
      --db string   Database path
  -v, --verbose     Enable verbose/debug output

Use "bd epic [command] --help" for more information about a command."#;

    #[test]
    fn test_parse_top_level_commands() {
        let (cmd, _globals, _groups) = parse_help_output(BD_HELP);

        assert_eq!(
            cmd.description,
            "Issues chained together like beads. A lightweight issue tracker with first-class dependency support."
        );

        // Should have commands from all groups
        assert!(cmd.subcommands.contains_key("children"));
        assert!(cmd.subcommands.contains_key("create"));
        assert!(cmd.subcommands.contains_key("count"));
        assert!(cmd.subcommands.contains_key("help"));
        assert!(cmd.subcommands.contains_key("version"));

        // Check descriptions
        assert_eq!(
            cmd.subcommands["create"].description,
            "Create a new issue (or multiple issues from markdown file)"
        );
    }

    #[test]
    fn test_parse_command_groups() {
        let (_cmd, _globals, groups) = parse_help_output(BD_HELP);

        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0].name, "Working With Issues");
        assert!(groups[0].commands.contains(&"create".to_string()));
        assert!(groups[0].commands.contains(&"close".to_string()));

        assert_eq!(groups[1].name, "Views & Reports");
        assert!(groups[1].commands.contains(&"count".to_string()));
        assert!(groups[1].commands.contains(&"status".to_string()));

        assert_eq!(groups[2].name, "Additional Commands");
    }

    #[test]
    fn test_parse_group_assignment() {
        let (cmd, _, _) = parse_help_output(BD_HELP);

        assert_eq!(
            cmd.subcommands["create"].group.as_deref(),
            Some("Working With Issues")
        );
        assert_eq!(
            cmd.subcommands["count"].group.as_deref(),
            Some("Views & Reports")
        );
    }

    #[test]
    fn test_parse_flags() {
        let (cmd, _globals, _) = parse_help_output(BD_HELP);

        let db_flag = cmd.flags.iter().find(|f| f.long == "db").unwrap();
        assert_eq!(db_flag.value_type.as_deref(), Some("string"));
        assert!(db_flag.description.contains("Database path"));
        assert_eq!(db_flag.default.as_deref(), Some("auto-discover .beads/*.db"));

        let verbose_flag = cmd.flags.iter().find(|f| f.long == "verbose").unwrap();
        assert_eq!(verbose_flag.short, Some('v'));
        assert_eq!(verbose_flag.value_type, None);

        let quiet_flag = cmd.flags.iter().find(|f| f.long == "quiet").unwrap();
        assert_eq!(quiet_flag.short, Some('q'));

        let help_flag = cmd.flags.iter().find(|f| f.long == "help").unwrap();
        assert_eq!(help_flag.short, Some('h'));
    }

    #[test]
    fn test_parse_bool_vs_string_flags() {
        let (cmd, _, _) = parse_help_output(BD_HELP);

        // --json is a bool flag (no value type)
        let json_flag = cmd.flags.iter().find(|f| f.long == "json").unwrap();
        assert_eq!(json_flag.value_type, None);

        // --db is a string flag
        let db_flag = cmd.flags.iter().find(|f| f.long == "db").unwrap();
        assert_eq!(db_flag.value_type.as_deref(), Some("string"));
    }

    #[test]
    fn test_parse_subcommand_with_aliases() {
        let (cmd, _globals, _) = parse_help_output(CREATE_HELP);

        assert_eq!(cmd.aliases, vec!["new"]);
    }

    #[test]
    fn test_parse_subcommand_flags() {
        let (cmd, globals, _) = parse_help_output(CREATE_HELP);

        // Local flags
        let priority = cmd.flags.iter().find(|f| f.long == "priority").unwrap();
        assert_eq!(priority.short, Some('p'));
        assert_eq!(priority.value_type.as_deref(), Some("string"));
        assert_eq!(priority.default.as_deref(), Some("2"));

        let labels = cmd.flags.iter().find(|f| f.long == "labels").unwrap();
        assert_eq!(labels.value_type.as_deref(), Some("strings"));

        // Global flags
        let db = globals.iter().find(|f| f.long == "db").unwrap();
        assert_eq!(db.value_type.as_deref(), Some("string"));
    }

    #[test]
    fn test_parse_subcommand_with_subcommands() {
        let (cmd, _, _) = parse_help_output(EPIC_HELP);

        assert_eq!(cmd.subcommands.len(), 2);
        assert!(cmd.subcommands.contains_key("close-eligible"));
        assert!(cmd.subcommands.contains_key("status"));
        assert_eq!(
            cmd.subcommands["close-eligible"].description,
            "Close epics where all children are complete"
        );
    }

    #[test]
    fn test_parse_usage() {
        let (cmd, _, _) = parse_help_output(CREATE_HELP);
        assert_eq!(cmd.usage.as_deref(), Some("bd create [title] [flags]"));
    }

    #[test]
    fn test_parse_flag_default_extraction() {
        let (cmd, _, _) = parse_help_output(CREATE_HELP);

        let type_flag = cmd.flags.iter().find(|f| f.long == "type").unwrap();
        assert_eq!(type_flag.default.as_deref(), Some("task"));

        // Flags without defaults
        let assignee = cmd.flags.iter().find(|f| f.long == "assignee").unwrap();
        assert_eq!(assignee.default, None);
    }
}
