use anyhow::{Context, Result};

use clap::{ArgMatches, Args, Command, FromArgMatches};
use dsi_bitstream::prelude::*;
use dsi_progress_logger::prelude::*;
use lender::Lender;
use std::{
    env::temp_dir,
    fs::File,
    io::{copy, BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
    time::Duration,
};
use webgraph::prelude::*;

pub const COMMAND_NAME: &str = "bin";

#[derive(Args, Debug)]
#[command(about = "Decompresses a BvGraph, to the binary \"bin\" format used by Zuckerli to compress graphs.", long_about = None)]
pub struct CliArgs {
    /// The basename of the source graph.
    pub src: PathBuf,
    /// The output path of the decompressed graph.
    pub dst: PathBuf,
}

// Simple on-disk representation of a graph that can directly mapped into memory
// (allowing reduced memory usage).
// Format description:
// - 8 bytes of fingerprint
// - 4 bytes to represent the number of nodes N
// - N+1 8-byte integers that represent the index of the first edge of the i-th
//   adjacency list. The last of these integers is the total number of edges, M.
// - M 4-byte integers that represent the destination node of each graph edge.
struct BinGraph {
    offsets: Vec<u64>,
    arcs: Vec<u32>,
}

impl BinGraph {
    fn add_list(&mut self, node_index: usize, successors: impl IntoIterator<Item = usize>) {
        assert_eq!(node_index, self.offsets.len());
        let total_arcs = self.arcs.len();
        self.offsets.push(total_arcs as u64);
        self.arcs.extend(successors.into_iter().map(|x| x as u32));
    }

    fn new() -> Self {
        BinGraph {
            offsets: Vec::new(),
            arcs: Vec::new(),
        }
    }

    fn from_file(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let file = File::open(path.as_ref()).with_context(|| {
            format!("Failed to open file {:?}", path.as_ref().to_string_lossy())
        })?;
        let mut reader = BufReader::new(file);

        // Read and verify fingerprint (8 bytes)
        let mut fingerprint_bytes = [0u8; 8];
        reader
            .read_exact(&mut fingerprint_bytes)
            .context("Failed to read fingerprint from file")?;
        let fingerprint = u64::from_le_bytes(fingerprint_bytes);

        // Expected fingerprint: (size_of::<u64>() << 4) | size_of::<u32>()
        let expected_fingerprint =
            (std::mem::size_of::<u64>() as u64) << 4 | std::mem::size_of::<u32>() as u64;
        anyhow::ensure!(
            fingerprint == expected_fingerprint,
            "Invalid fingerprint: expected {}, got {}",
            expected_fingerprint,
            fingerprint
        );

        // Read number of nodes (4 bytes)
        let mut num_nodes_bytes = [0u8; 4];
        reader
            .read_exact(&mut num_nodes_bytes)
            .context("Failed to read number of nodes")?;
        let num_nodes = u32::from_le_bytes(num_nodes_bytes) as usize;

        // Read offsets (N+1 8-byte integers)
        let mut offsets = Vec::with_capacity(num_nodes);
        let mut total_arcs = 0u64;
        for i in 0..=num_nodes {
            let mut offset_bytes = [0u8; 8];
            reader
                .read_exact(&mut offset_bytes)
                .context("Failed to read offset")?;
            let offset = u64::from_le_bytes(offset_bytes);

            if i == num_nodes {
                // The last offset is the total number of arcs
                total_arcs = offset;
            } else {
                offsets.push(offset);
            }
        }

        // Read arcs (M 4-byte integers) and verify each arc is < num_nodes
        let mut arcs = Vec::with_capacity(total_arcs as usize);
        for _ in 0..total_arcs {
            let mut arc_bytes = [0u8; 4];
            reader
                .read_exact(&mut arc_bytes)
                .context("Failed to read arc destination")?;
            let arc = u32::from_le_bytes(arc_bytes) as usize;

            anyhow::ensure!(
                arc < num_nodes,
                "Invalid arc destination: {} >= number of nodes ({})",
                arc,
                num_nodes
            );
            arcs.push(arc as u32);
        }

        Ok(BinGraph { offsets, arcs })
    }

    fn write(&self, path: PathBuf) -> anyhow::Result<()> {
        // create the file if not exists
        let file = File::create(&path)?;
        let mut writer = BufWriter::new(file);
        // Fingerprint of the simple uncompressed graph format: number of bytes to
        // represent the number of edges followed by number of bytes to represent the
        // number of nodes.
        let fingerprint =
            (std::mem::size_of::<u64>() as u64) << 4 | std::mem::size_of::<u32>() as u64;
        // write the fingerprint
        writer.write_all(&fingerprint.to_le_bytes())?;
        // write the total number of nodes
        let number_of_nodes = self.offsets.len() as u32;
        writer.write_all(&number_of_nodes.to_le_bytes())?;
        // adds to the offsets the total number of arcs
        // write the offsets
        for offset in self.offsets.iter() {
            writer.write_all(&offset.to_le_bytes())?;
        }
        let total_arcs = self.arcs.len() as u64;
        writer.write_all(&total_arcs.to_le_bytes())?;
        // write the nodes
        for node in self.arcs.iter() {
            writer.write_all(&node.to_le_bytes())?;
        }
        writer.flush()?;
        Ok(())
    }
}

/// Writer for large bin (uncompressed) graphs: it allows the adjacency list of the next node
/// to be added incrementally, but, unlike the implementation in `BinGraph`, it writes both the
/// offsets and the arcs to two separate files created in the `temp_path` during the instantiation of the object.
/// Finally, it is possible to write the entire graph to a single file that conforms to the
/// bin graph file specification.
struct BinGraphWriter {
    offset_file: PathBuf,
    arc_file: PathBuf,
    offset_writer: BufWriter<File>,
    arc_writer: BufWriter<File>,
    node_count: u64,
    arc_count: u64,
}

impl BinGraphWriter {
    fn new(temp_path: &Path) -> Self {
        let offset_file = temp_path.join("offsets.tmp");
        let arc_file = temp_path.join("arcs.tmp");

        let offset_writer =
            BufWriter::new(File::create(&offset_file).expect("Failed to create offset file"));
        let arc_writer =
            BufWriter::new(File::create(&arc_file).expect("Failed to create arc file"));

        BinGraphWriter {
            offset_file,
            arc_file,
            offset_writer,
            arc_writer,
            arc_count: 0,
            node_count: 0,
        }
    }

    fn add_list(&mut self, successors: impl IntoIterator<Item = usize>) -> anyhow::Result<()> {
        // Write current arc count as offset for this node
        self.offset_writer
            .write_all(&self.arc_count.to_le_bytes())
            .context("Failed to write offset")?;

        // Write all successors and update arc count
        for successor in successors {
            self.arc_writer
                .write_all(&(successor as u32).to_le_bytes())
                .context("Failed to write arc")?;
            self.arc_count += 1;
        }
        self.node_count += 1;

        Ok(())
    }

    fn write(mut self, path: PathBuf) -> anyhow::Result<()> {
        create_parent_dir(&path)?;

        // Flush and close temporary file writers
        self.offset_writer
            .flush()
            .context("Failed to flush offset writer")?;
        self.arc_writer
            .flush()
            .context("Failed to flush arc writer")?;
        drop(self.offset_writer);
        drop(self.arc_writer);

        let mut output_file = BufWriter::new(File::create(&path)?);

        // Fingerprint of the simple uncompressed graph format
        let fingerprint =
            (std::mem::size_of::<u64>() as u64) << 4 | std::mem::size_of::<u32>() as u64;
        output_file
            .write_all(&fingerprint.to_le_bytes())
            .context("Failed to write fingerprint")?;

        // Write number of nodes
        let num_nodes = self.node_count as u32;
        output_file
            .write_all(&num_nodes.to_le_bytes())
            .context("Failed to write number of nodes")?;

        // Copy offsets file in buffered chunks
        let offset_input = File::open(&self.offset_file).context("Failed to open offsets file")?;
        let mut offset_reader = BufReader::new(offset_input);
        copy(&mut offset_reader, &mut output_file).context("Failed to copy offsets")?;

        // Write total arc count
        output_file
            .write_all(&self.arc_count.to_le_bytes())
            .context("Failed to write total arc count")?;

        // Copy arcs file in buffered chunks
        let arc_input = File::open(&self.arc_file).context("Failed to open arcs file")?;
        let mut arc_reader = BufReader::new(arc_input);
        copy(&mut arc_reader, &mut output_file).context("Failed to copy arcs")?;

        output_file.flush().context("Failed to flush output file")?;

        // Clean up temp files
        std::fs::remove_file(&self.offset_file).context("Failed to remove offset temp file")?;
        std::fs::remove_file(&self.arc_file).context("Failed to remove arc temp file")?;

        Ok(())
    }
}

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

    let mut pl = ProgressLogger::default();
    // log every five minutes
    pl.log_interval(Duration::from_secs(60));

    pl.item_name("node")
        .expected_updates(Some(graph.num_nodes()));

    let temp_dir = &tempfile::tempdir()?;
    let mut bin = BinGraphWriter::new(temp_dir.path());
    let mut iter = graph.iter();
    while let Some((true_node_id, true_succ)) = iter.next() {
        bin.add_list(true_succ)?;
        pl.update();
    }
    pl.done();
    bin.write(args.dst)?;

    log::info!(
        "The re-compression took {:.3} seconds",
        start.elapsed().as_secs_f64()
    );
    Ok(())
}
