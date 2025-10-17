use anyhow::Result;
use clap::Command;

pub mod analyze;
pub mod run;
pub mod to;

pub fn main() -> Result<()> {
    let args = std::env::args_os();
    let command = Command::new("webgraph-tools")
        .about("Webgraph tools for minor operations on webgraph files.")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .after_help(
            "Environment (noteworthy environment variables used):
RUST_MIN_STACK: minimum thread stack size (in bytes)
TMPDIR: where to store temporary files (potentially very large ones)
",
        );

    let command = to::cli(command);
    let command = analyze::cli(command);
    let command = run::cli(command);
    let command = command.display_order(0); // sort args alphabetically
    let mut completion_command = command.clone();
    let matches = command.get_matches_from(args);
    let subcommand = matches.subcommand();
    // if no command is specified, print the help message
    if subcommand.is_none() {
        completion_command.print_help().unwrap();
        return Ok(());
    }
    match subcommand.unwrap() {
        (to::COMMAND_NAME, sub_m) => to::main(sub_m),
        (run::COMMAND_NAME, sub_m) => run::main(sub_m),
        (analyze::COMMAND_NAME, sub_m) => analyze::main(sub_m),
        (command_name, _) => {
            // this shouldn't happen as clap should catch this
            eprintln!("Unknown command: {:?}", command_name);
            completion_command.print_help().unwrap();
            std::process::exit(1);
        }
    }
}
