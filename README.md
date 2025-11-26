# `clstr`

A small crate to parse and write `.clstr` files. Parses the standard CD-HIT ``.clstr` format as produced by `cd-hit`, `cd-hit-est`, `cd-hit-2d`, `cd-hit-est-2d`, and related tools. Supports both amino-acid (aa) and nucleotide (nt) clusters, and tolerates identity fields like at 99.9%, at 99.9%/100%, and at -/100%.

## API 

A really simple example which just reads in a file and prints it.

```rust
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;

fn main() -> Result<(), std::error::Error> {
    // Define the input and output file paths
    let input_path = Path::new("input.clstr");
    let output_path = Path::new("output.clstr");

    // make the parser
    let parser = clstr::from_path(input_path)?;

    // and the writer
    let clstr_writer = clstr::to_path(output_path)?;

    for cluster_res in parser {
      let cluster = cluster_res?;
      clstr_writer.write_cluster(&cluster)?;
    }

    clstr_writer.flush()?;

    Ok(())
}
```

## Binaries

TODO, describe binaries.
