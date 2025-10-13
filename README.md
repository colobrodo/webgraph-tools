# WebGraph Tools
This is a collection of tools related to webgraph.

## Current tools
- `to bin`: Convert the BvGraph file (passed as basename) to an uncompressed "bin" format used also by Zuckerli and other tools. 
- `dissect`: Print how many bits are used by each component of the graph. 
- `run rgb`: Return a permutation (in a webgraph-compatible format) for the graph using the Recursive Graph Bisection algorithm, based on the implementation https://github.com/mpetri/faster-graph-bisection from the paper *\"Faster Index Reordering with Bipartite Graph Partitioning by Joel Mackenzie, Matthias Petri, and Alistair Moffat\"*.
   In order to compile the code using this command you should specify the `GAIN` env variable during the compilation to choose a gain approximation function (choose between `approx_1`, `default` or `approx_2`).
   For example:
   ```
   GAIN=approx_1 cargo build --release
   ```
