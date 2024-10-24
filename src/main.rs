// clstr
// part of `vscan`
// at the moment, this simply takes a clstr file and writes the top 500 clusters

use std::path::PathBuf;

use clap::{crate_version, value_parser, Arg, ArgAction, ArgMatches, Command};
use clstr::{Cluster, Result as ClstrResult};

fn parse_args() -> ArgMatches {
    Command::new("clstr")
        .version(crate_version!())
        .arg_required_else_help(true)
        .next_line_help(true)
        .help_expected(true)
        .max_term_width(80)
        .arg(
            Arg::new("FILE")
                .help("The input file in `.clstr` format.")
                .id("FILE")
                .value_parser(value_parser!(PathBuf))
                .required(true)
                .index(1),
        )
        .get_matches()
}

fn main() -> ClstrResult<()> {
    let matches = parse_args();

    let clstr_file = matches.get_one::<PathBuf>("FILE").unwrap().clone();

    let parser = clstr::from_path(clstr_file.clone())?;

    let cluster_number = 500;

    // get all the clusters from the parser, sort them by cluster size, with
    // largest first
    let clusters: ClstrResult<Vec<Cluster>> = parser.into_iter().collect();
    let mut clusters = clusters?;
    clusters.sort_by_key(|b| std::cmp::Reverse(b.size()));

    // now filter to get the top cluster_number clusters
    let clusters = clusters.into_iter().take(cluster_number);

    // and write these to file
    let mut out_file = clstr::to_path(clstr_file.with_extension("top"))?;
    for cluster in clusters {
        out_file.write_cluster(&cluster)?;
    }

    Ok(())
}
