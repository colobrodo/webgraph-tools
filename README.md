# WebGraph Tools
This is a collection of tools related to WebGraph [1, 3].

## Current tools
- `to bin`: Convert the BvGraph file (passed as basename) to an uncompressed "bin" format used also by Zuckerli [2] and other tools. 
- `dissect`: Print how many bits are used by each component of the graph. 
- `run rgb`: Return a permutation (in a webgraph-compatible format) for the graph using the Recursive Graph Bisection algorithm, that uses [4].
   In order to compile the code using this command you should specify the `GAIN` env variable during the compilation to choose a gain approximation function (choose between `approx_1`, `default` or `approx_2`).
   For example:
   ```
   GAIN=approx_1 cargo build --release
   ```

## References

_[1] Paolo Boldi and Sebastiano Vigna. "The WebGraph Framework I: Compression Techniques." In: Thirteenth International World Wide Web Conference Proceedings, WWW2004 (Apr. 2004). doi: 10.1145/988672.988752._   

_[2] Luca Versari, Iulia-Maria Comsa, Alessio Conte, Roberto Grossi: Zuckerli: A New Compressed Representation for Graphs (https://github.com/google/zuckerli) IEEE Access 8: 219233-219243 (2020)._   

_[3] Tommaso Fontana, Sebastiano Vigna, and Stefano Zacchiroli. “WebGraph: The Next Generation (Is in Rust)”. In: Companion Proceedings of the ACM on Web Conference 2024 (2024), pp. 686–689. doi: 10.1145/3589335.3651581._

_[3] Joel Mackenzie, Matthias Petri, and Alistair Moffat. 
"Faster Index Reordering with Bipartite Graph Partitioning" (https://github.com/mpetri/faster-graph-bisection) In: SIGIR '21: Proceedings of the 44th International ACM SIGIR Conference on Research and Development in Information Retrieval (2021), doi: 10.1145/3404835.3462991._