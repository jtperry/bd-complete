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

use std::collections::BTreeMap;

/// A flag for a CLI command (e.g., --verbose, -v).
#[derive(Debug, Clone, PartialEq)]
pub struct Flag {
    /// Long form, e.g. "verbose"
    pub long: String,
    /// Short form, e.g. Some('v')
    pub short: Option<char>,
    /// Human-readable description
    pub description: String,
    /// The value type if the flag takes an argument (e.g. "string", "int", "strings").
    /// None for boolean flags.
    pub value_type: Option<String>,
    /// Default value, if any
    pub default: Option<String>,
}

/// A command group/category (e.g., "Working With Issues", "Views & Reports").
#[derive(Debug, Clone, PartialEq)]
pub struct CommandGroup {
    pub name: String,
    pub commands: Vec<String>,
}

/// A single command node in the tree.
#[derive(Debug, Clone, PartialEq)]
pub struct Command {
    /// The command name (e.g. "create", "epic")
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Aliases for this command (e.g. ["new"] for "create")
    pub aliases: Vec<String>,
    /// Usage string from help output
    pub usage: Option<String>,
    /// Flags local to this command
    pub flags: Vec<Flag>,
    /// Subcommands keyed by name
    pub subcommands: BTreeMap<String, Command>,
    /// Which group/category this command belongs to (from parent's help)
    pub group: Option<String>,
}

impl Command {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            aliases: Vec::new(),
            usage: None,
            flags: Vec::new(),
            subcommands: BTreeMap::new(),
            group: None,
        }
    }
}

/// The root of the parsed command tree.
#[derive(Debug, Clone, PartialEq)]
pub struct CommandTree {
    /// The root command (e.g., "bd")
    pub root: Command,
    /// Global flags that apply to all subcommands
    pub global_flags: Vec<Flag>,
    /// Command groups discovered at the top level
    pub groups: Vec<CommandGroup>,
}

impl CommandTree {
    pub fn new(root: Command) -> Self {
        Self {
            root,
            global_flags: Vec::new(),
            groups: Vec::new(),
        }
    }
}
