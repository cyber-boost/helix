use clap_complete::Shell;
use std::io::Cursor;

pub fn completions_command(
    shell: Shell,
    verbose: bool,
    quiet: bool,
) -> String {
    if !quiet && !verbose {
        println!("Generating shell completions...");
    }

    use clap_complete::{generate, shells::{Bash, Zsh, Fish, PowerShell, Elvish}};

    // We need to import the CLI command from the main binary
    // For now, we'll create a simple placeholder command
    use clap::Command;
    let mut cmd = Command::new("hlx")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("add")
                .about("Add a dependency")
        )
        .subcommand(
            Command::new("build")
                .about("Build the project")
        )
        .subcommand(
            Command::new("run")
                .about("Run the project")
        );

    let mut buf = Cursor::new(Vec::new());

    match shell {
        Shell::Bash => {
            generate(Bash, &mut cmd, "hlx", &mut buf);
        }
        Shell::Zsh => {
            generate(Zsh, &mut cmd, "hlx", &mut buf);
        }
        Shell::Fish => {
            generate(Fish, &mut cmd, "hlx", &mut buf);
        }
        Shell::PowerShell => {
            generate(PowerShell, &mut cmd, "hlx", &mut buf);
        }
        Shell::Elvish => {
            generate(Elvish, &mut cmd, "hlx", &mut buf);
        }
        _ => {
            if !quiet {
                eprintln!("Unsupported shell: {:?}", shell);
            }
            return String::new();
        }
    }

    let completions = String::from_utf8_lossy(&buf.into_inner()).to_string();

    if verbose && !quiet {
        println!("✅ Shell completions generated for {:?}", shell);
    }

    completions
}