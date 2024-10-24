// clstr
// part of `vscan`
// at the moment, this simply takes a clstr file and writes the top N clusters

use std::{
    collections::HashMap,
    fs::File,
    path::{Path, PathBuf},
};

use bio::io::fasta;
use clap::{crate_version, value_parser, Arg, ArgMatches, Command};
use clstr::{Cluster, Result as ClstrResult};

fn parse_args() -> ArgMatches {
    Command::new("clstr")
        .version(crate_version!())
        .arg_required_else_help(true)
        .next_line_help(true)
        .help_expected(true)
        .max_term_width(80)
        .subcommand_required(true)
        .subcommand(
            Command::new("topn")
                .about("Write the top N clusters to a new file.")
                .arg(
                    Arg::new("FILE")
                        .help("The input file in `.clstr` format.")
                        .id("FILE")
                        .value_parser(value_parser!(PathBuf))
                        .required(true)
                        .index(1),
                )
                .arg(
                    Arg::new("cluster-number")
                        .help("The number of top clusters to write to the output file.")
                        .id("cluster-number")
                        .short('n')
                        .long("cluster-number")
                        .num_args(1)
                        .value_parser(value_parser!(usize))
                        .default_value("500"),
                ),
        )
        .subcommand(
            Command::new("tofasta")
                .about("Generate multiple fasta files given an input cluster file.")
                .arg(
                    Arg::new("FILE")
                        .help("The input file in `.clstr` format.")
                        .id("FILE")
                        .value_parser(value_parser!(PathBuf))
                        .required(true)
                        .num_args(1)
                        .index(1),
                )
                .arg(
                    Arg::new("DATABASE")
                        .help("The database file containing sequences, from which the cluster file was derived.")
                        .id("DATABASE")
                        .value_parser(value_parser!(PathBuf))
                        .required(true)
                        .num_args(1)
                        .index(2)
                )
        )
        .get_matches()
}

fn top_n(matches: &ArgMatches) -> ClstrResult<()> {
    let clstr_file = matches.get_one::<PathBuf>("FILE").unwrap().clone();
    let cluster_number = *matches.get_one::<usize>("cluster-number").unwrap();

    let parser = clstr::from_path(clstr_file.clone())?;

    // get all the clusters from the parser, sort them by cluster size, with
    // largest first
    let clusters: ClstrResult<Vec<Cluster>> = parser.into_iter().collect();
    let mut clusters = clusters?;
    clusters.sort_by_key(|b| std::cmp::Reverse(b.size()));

    // now filter to get the top cluster_number clusters
    let clusters = clusters.into_iter().take(cluster_number);

    // and write these to file
    let mut out_file =
        clstr::to_path(clstr_file.with_extension(format!("top{}.clstr", cluster_number)))?;
    for cluster in clusters {
        out_file.write_cluster(&cluster)?;
    }

    Ok(())
}

/// A function to read the FASTA file and return a map of sequence ID to sequence data.
fn read_fasta<P: AsRef<Path> + std::fmt::Debug>(
    fasta_path: P,
) -> ClstrResult<HashMap<String, (String, String)>> {
    let mut fasta_map = HashMap::new();

    let records = fasta::Reader::from_file(fasta_path)
        .map_err(|e| {
            clstr::Error::from(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            ))
        })?
        .records();

    for record in records {
        let rec = record?;
        let desc = rec.desc().unwrap_or("");

        let seq = String::from_utf8(rec.seq().to_owned()).unwrap();
        fasta_map.insert(rec.id().to_string(), (desc.to_string(), seq));
    }

    Ok(fasta_map)
}

fn to_fasta(matches: &ArgMatches) -> ClstrResult<()> {
    let clstr_file = matches.get_one::<PathBuf>("FILE").unwrap().clone();
    let database_file = matches.get_one::<PathBuf>("DATABASE").unwrap().clone();

    let fasta_map = read_fasta(database_file)?;

    let cluster_parser = clstr::from_path(clstr_file.clone())?;

    for cluster in cluster_parser {
        let cluster = cluster?;
        let cluster_id = cluster.cluster_id();
        let out_file = File::create(clstr_file.with_extension(format!("{}.fasta", cluster_id)))?;
        write_cluster_to_fasta(&cluster, &fasta_map, out_file)?;
    }

    Ok(())
}

/// Writes sequences from a cluster into a FASTA file.
fn write_cluster_to_fasta<P: std::io::Write>(
    cluster: &Cluster,
    fasta_map: &HashMap<String, (String, String)>,
    output_path: P,
) -> ClstrResult<()> {
    let mut writer = fasta::Writer::new(output_path);

    for sequence in cluster.sequences() {
        if let Some((id, (desc, fasta_sequence))) = fasta_map.get_key_value(sequence.id()) {
            let record = fasta::Record::with_attrs(id, Some(desc), fasta_sequence.as_bytes());
            writer.write_record(&record)?;
        } else {
            eprintln!("Warning: sequence ID {} not found in FASTA", sequence.id());
        }
    }

    Ok(())
}

fn main() -> ClstrResult<()> {
    let matches = parse_args();

    match matches.subcommand() {
        Some(("topn", matches)) => top_n(matches)?,
        Some(("tofasta", matches)) => to_fasta(matches)?,
        _ => unreachable!("Exhausted list of subcommands and subcommand_required prevents `None`"),
    }

    Ok(())
}
