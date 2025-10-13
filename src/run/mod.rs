use anyhow::Result;
use clap::{ArgMatches, Command};

pub mod rgb;

pub const COMMAND_NAME: &str = "run";

pub fn cli(command: Command) -> Command {
    let sub_command = Command::new(COMMAND_NAME)
        .about("Long running computation on graphs.")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .allow_external_subcommands(true);
    let sub_command = rgb::cli(sub_command);
    command.subcommand(sub_command.display_order(0))
}

pub fn main(submatches: &ArgMatches) -> Result<()> {
    match submatches.subcommand() {
        Some((rgb::COMMAND_NAME, sub_m)) => rgb::main(sub_m),
        Some((command_name, _)) => {
            eprintln!("Unknown command: {:?}", command_name);
            std::process::exit(1);
        }
        None => {
            eprintln!("No command given for to");
            std::process::exit(1);
        }
    }
}
