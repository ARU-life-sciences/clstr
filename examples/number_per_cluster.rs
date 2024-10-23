extern crate clstr;

use clstr::parse_clstr;
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // use the first arg from the cli
    let clstr_file = std::env::args()
        .nth(1)
        .expect("Please provide a .clstr file");

    // Parse the .clstr file using the parser
    let parser = parse_clstr(clstr_file)?;

    // write to stdout using std::io
    let mut stdout = std::io::stdout();

    // Iterate through each cluster and print the cluster ID and sequence count
    for cluster in parser {
        let cluster = cluster?;
        let _ = writeln!(
            stdout,
            "Cluster {}: {} sequences",
            cluster.cluster_id(),
            cluster.size()
        );
    }

    Ok(())
}
