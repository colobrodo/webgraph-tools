use anyhow::Result;
use clap::{ArgMatches, Command};

mod dec_stats_and_count;
pub mod dissect;
use dec_stats_and_count::*;

pub const COMMAND_NAME: &str = "analyze";

pub fn cli(command: Command) -> Command {
    let sub_command = Command::new(COMMAND_NAME)
        .about("Commands usefull to analyze and debug the encoding of a graph.")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .allow_external_subcommands(true);
    let sub_command = dissect::cli(sub_command);
    command.subcommand(sub_command.display_order(0))
}

pub fn main(submatches: &ArgMatches) -> Result<()> {
    match submatches.subcommand() {
        Some((dissect::COMMAND_NAME, sub_m)) => dissect::main(sub_m),
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
