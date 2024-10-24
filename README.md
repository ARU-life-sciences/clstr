# `clstr`

A small crate to parse and write `.clstr` files.

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

There are two programs `clstr topn` and `clstr tofasta` which are used in `vscan`.
