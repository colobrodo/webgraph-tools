use anyhow::Result;
use clap::{ArgMatches, Args, Command, FromArgMatches};
use dsi_bitstream::{dispatch::factory::CodesReaderFactoryHelper, prelude::*};
use dsi_progress_logger::prelude::*;
use lender::*;
use std::path::PathBuf;
use webgraph::prelude::*;

use crate::analyze::{component::BvGraphComponent, ConsumerDecoderFactory};

pub const COMMAND_NAME: &str = "print";

#[derive(Args, Debug)]
#[command(about = "Prints the decoded instantaneous codes from the stream to stdout. Usefull to inspect the distribution of gaps or any other component", long_about = None)]
pub struct CliArgs {
    /// The component to print
    pub component: BvGraphComponent,
    /// The basename of the graph.
    pub src: PathBuf,
}

pub fn cli(command: Command) -> Command {
    command.subcommand(CliArgs::augment_args(Command::new(COMMAND_NAME)).display_order(0))
}

pub fn main(submatches: &ArgMatches) -> Result<()> {
    let args = CliArgs::from_arg_matches(submatches)?;

    match get_endianness(&args.src)?.as_str() {
        BE::NAME => log_graph::<BE>(args),
        LE::NAME => log_graph::<LE>(args),
        e => panic!("Unknown endianness: {}", e),
    }
}

pub fn log_graph<E: Endianness + 'static>(args: CliArgs) -> Result<()>
where
    MmapHelper<u32>: CodesReaderFactoryHelper<E>,
    for<'a> LoadModeCodesReader<'a, E, Mmap>: BitSeek,
{
    let target_component = args.component;
    let graph = BvGraphSeq::with_basename(args.src)
        .endianness::<E>()
        .load()?
        .map_factory(|factory| {
            ConsumerDecoderFactory::new(factory, move |component, value| {
                if component == target_component {
                    println!("{}", value)
                }
            })
        });

    let mut pl = ProgressLogger::default();
    pl.display_memory(true)
        .item_name("node")
        .expected_updates(Some(graph.num_nodes()));

    pl.start("Scanning...");

    let mut iter = graph.iter();
    while iter.next().is_some() {
        pl.light_update();
    }
    pl.done();

    Ok(())
}
