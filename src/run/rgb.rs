use anyhow::{Context, Result};

use clap::{ArgMatches, Args, Command, FromArgMatches};
use dsi_bitstream::prelude::*;
use dsi_progress_logger::prelude::*;
use lender::Lender;
use std::{
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    time::Duration,
};
use webgraph::prelude::*;

pub const COMMAND_NAME: &str = "rgb";

#[derive(Args, Debug)]
#[command(
    about = "Reorder the graph using the Recursive Graph Bisection algorithm",
    long_about = "Reorder the graph using the Recursive Graph Bisection algorithm,
 based on the implementation https://github.com/mpetri/faster-graph-bisection from the paper
 \"Faster Index Reordering with Bipartite Graph Partitioning by Joel Mackenzie, Matthias Petri, and Alistair Moffat\"."
)]
pub struct CliArgs {
    /// The basename of the source graph.
    pub src: PathBuf,
    /// The output path of the permutation calculated by recursive graph bisection.
    pub dst: PathBuf,

    /// Max swap iteration
    #[arg(short, long, default_value = "20")]
    iterations: usize,

    /// Maximum recursive depth
    #[arg(long, default_value = "100")]
    max_depth: usize,

    /// Minimum partition size
    #[arg(short, long, default_value = "16")]
    min_partition_size: usize,

    /// Sort the leafs by identifier id
    #[arg(long)]
    sort_leaf: bool,
}

// TODO: this functions are duplicated from webgraph but they are not exposed.
//       duplicate them to keep a semantic as close as possible to webgraph graphs
/// Creates all parent directories of the given file path.
fn create_parent_dir(file_path: impl AsRef<Path>) -> Result<()> {
    // ensure that the dst directory exists
    if let Some(parent_dir) = file_path.as_ref().parent() {
        std::fs::create_dir_all(parent_dir).with_context(|| {
            format!(
                "Failed to create the directory {:?}",
                parent_dir.to_string_lossy()
            )
        })?;
    }
    Ok(())
}

fn store_perm(data: &[usize], perm: impl AsRef<Path>) -> Result<()> {
    let mut file = std::fs::File::create(&perm).with_context(|| {
        format!(
            "Could not create permutation at {}",
            perm.as_ref().display()
        )
    })?;
    let mut buf = BufWriter::new(&mut file);
    for word in data.iter() {
        buf.write_all(&word.to_be_bytes()).with_context(|| {
            format!("Could not write permutation to {}", perm.as_ref().display())
        })?;
    }
    Ok(())
}

pub fn cli(command: Command) -> Command {
    command.subcommand(CliArgs::augment_args(Command::new(COMMAND_NAME)).display_order(0))
}

pub fn main(submatches: &ArgMatches) -> Result<()> {
    let start = std::time::Instant::now();
    let args = CliArgs::from_arg_matches(submatches)?;

    create_parent_dir(&args.dst)?;

    let graph = BvGraphSeq::with_basename(&args.src)
        .endianness::<BE>()
        .load()?;

    let mut pl = progress_logger!(
        display_memory = true,
        log_interval = Duration::from_secs(5 * 60)
    );

    pl.item_name("node")
        .expected_updates(Some(graph.num_nodes()));
    pl.start(format!(
        "Building the collection with {} nodes...",
        graph.num_nodes()
    ));

    let mut documents = Vec::with_capacity(graph.num_nodes());
    let mut iter = graph.iter();
    while let Some((node_id, succs)) = iter.next() {
        let doc = rgb::forward::Doc {
            postings: Vec::from_iter(succs.map(|succ_id| (succ_id as _, 1u32))),
            org_id: node_id as _,
            gain: 0.0,
            leaf_id: -1,
        };
        documents.push(doc);
        pl.update();
    }
    pl.done();

    documents.sort_by(|a, b| b.postings.len().cmp(&a.postings.len()));
    let num_non_empty = documents
        .iter()
        .position(|d| d.postings.is_empty())
        .unwrap_or(documents.len());
    log::info!("{} lists not empty", num_non_empty);

    pl.item_name("rgb");
    pl.start("Running Recursive Graph Bisection");
    rgb::recursive_graph_bisection(
        &mut documents[..num_non_empty],
        graph.num_nodes(),
        args.iterations,
        args.min_partition_size,
        args.max_depth,
        1, // starting depth
        args.sort_leaf,
    );

    let mut perm = vec![0; graph.num_nodes()];
    for (new_id, document) in documents.iter().enumerate() {
        perm[document.org_id as usize] = new_id;
    }
    store_perm(&perm, args.dst)?;
    pl.done();

    log::info!(
        "Recursive Graph Bisection took {:.3} seconds",
        start.elapsed().as_secs_f64()
    );
    Ok(())
}
