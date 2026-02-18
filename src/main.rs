pub mod bash;
pub mod command_tree;
pub mod parser;

use bash::generate_bash_completion;
use parser::build_command_tree;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::process;

fn print_usage() {
    eprintln!("Usage: bd-complete generate --shell <SHELL> [--output <FILE>]");
    eprintln!();
    eprintln!("Commands:");
    eprintln!("  generate    Generate a shell completion script");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --shell <SHELL>    Shell type: bash");
    eprintln!("  --output <FILE>    Write to file instead of stdout");
    eprintln!("  --help             Show this help");
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() || args.iter().any(|a| a == "--help" || a == "-h") {
        print_usage();
        if args.is_empty() {
            process::exit(1);
        }
        return;
    }

    if args[0] != "generate" {
        eprintln!("Error: unknown command '{}'. Expected 'generate'.", args[0]);
        eprintln!();
        print_usage();
        process::exit(1);
    }

    let mut shell: Option<String> = None;
    let mut output: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--shell" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("Error: --shell requires a value");
                    process::exit(1);
                }
                shell = Some(args[i].clone());
            }
            "--output" | "-o" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("Error: --output requires a value");
                    process::exit(1);
                }
                output = Some(args[i].clone());
            }
            other => {
                eprintln!("Error: unknown option '{other}'");
                process::exit(1);
            }
        }
        i += 1;
    }

    let shell = match shell {
        Some(s) => s,
        None => {
            eprintln!("Error: --shell is required");
            eprintln!();
            print_usage();
            process::exit(1);
        }
    };

    if shell != "bash" {
        eprintln!("Error: unsupported shell '{shell}'. Supported: bash");
        process::exit(1);
    }

    let tree = match build_command_tree("bd") {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Error building command tree: {e}");
            process::exit(1);
        }
    };

    let result = match output {
        Some(path) => {
            let file = File::create(&path).unwrap_or_else(|e| {
                eprintln!("Error creating file '{path}': {e}");
                process::exit(1);
            });
            let mut writer = BufWriter::new(file);
            generate_bash_completion(&tree, &mut writer)
                .and_then(|_| writer.flush())
        }
        None => {
            let stdout = io::stdout();
            let mut writer = BufWriter::new(stdout.lock());
            generate_bash_completion(&tree, &mut writer)
                .and_then(|_| writer.flush())
        }
    };

    if let Err(e) = result {
        eprintln!("Error generating completion script: {e}");
        process::exit(1);
    }
}
