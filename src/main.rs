// clstr
// part of `vscan`
// A tool for processing `.clstr` files produced by CD-HIT.

// currently functionality:
// - `topn`: write the top N clusters to a new file.
// - `filtern`: write clusters with at least N records to a new file.
// - `tofasta`: generate multiple fasta files given an input cluster file.
// - `stats`: get statistics on a CD-HIT cluster file.

use std::{collections::HashMap, fs::File, path::PathBuf};

use bio::io::fasta;
use clap::{crate_version, value_parser, Arg, ArgAction, ArgMatches, Command};
use clstr::{Cluster, Result as ClstrResult};
use flate2::read::GzDecoder;
use std::io::{BufReader, Read, Write};

fn parse_args() -> ArgMatches {
    Command::new("clstr")
        .version(crate_version!())
        .arg_required_else_help(true)
        .next_line_help(true)
        .help_expected(true)
        .max_term_width(80)
        .subcommand_required(true)
        .subcommand(
            Command::new("stats")
                .about("Get statistics on a CD-HIT cluster file.")
                .arg(
                    Arg::new("FILE")
                        .help("The input file in `.clstr` format.")
                        .id("FILE")
                        .value_parser(value_parser!(PathBuf))
                        .required(true)
                        .index(1),
                )
                .arg(
                    Arg::new("table")
                        .help("Print each cluster and number of sequences per cluster")
                        .id("table")
                        .short('t')
                        .long("table")
                        .action(ArgAction::SetTrue)
                )
        )
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
            Command::new("filtern")
                .about("Write clusters with at least N records to a new file.")
                .arg(
                    Arg::new("FILE")
                        .help("The input file in `.clstr` format.")
                        .id("FILE")
                        .value_parser(value_parser!(PathBuf))
                        .required(true)
                        .index(1),
                )
                .arg(
                    Arg::new("filter-number")
                        .help("The minimum number of sequences in a cluster for it to be written to the output file.")
                        .id("filter-number")
                        .short('n')
                        .long("filter-number")
                        .num_args(1)
                        .value_parser(value_parser!(usize))
                        .default_value("20"),
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
                        .help("The database file containing sequences, from which the cluster file was derived. Gzipped or not.")
                        .id("DATABASE")
                        .value_parser(value_parser!(PathBuf))
                        .required(true)
                        .num_args(1)
                        .index(2)
                )
        )
        .get_matches()
}

fn filter_n(matches: &ArgMatches) -> ClstrResult<()> {
    let clstr_file = matches.get_one::<PathBuf>("FILE").unwrap().clone();
    let filter_threshold = *matches.get_one::<usize>("filter-number").unwrap();

    let parser = clstr::from_path(clstr_file.clone())?;

    let mut out_file =
        clstr::to_path(clstr_file.with_extension(format!("more_than_{filter_threshold}.clstr")))?;
    for cluster in parser {
        let cluster = cluster?;

        if cluster.size() >= filter_threshold {
            out_file.write_cluster(&cluster)?;
        }
    }

    Ok(())
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
        clstr::to_path(clstr_file.with_extension(format!("top{cluster_number}.clstr")))?;
    for cluster in clusters {
        out_file.write_cluster(&cluster)?;
    }

    Ok(())
}

/// A function to read the FASTA file and return a map of sequence ID to sequence data.
fn read_fasta(fasta_path: PathBuf) -> ClstrResult<HashMap<String, (String, String)>> {
    let mut fasta_map = HashMap::new();

    let reader: Box<dyn Read> = if fasta_path.extension().and_then(|s| s.to_str()) == Some("gz") {
        // If the file is gzipped, use GzDecoder
        let file = File::open(fasta_path.clone())?;
        Box::new(GzDecoder::new(file))
    } else {
        // Otherwise, use a regular file reader
        let file = File::open(fasta_path.clone())?;
        Box::new(BufReader::new(file))
    };

    let records = fasta::Reader::new(reader).records();

    for record in records {
        let rec = record?;
        let desc = rec.desc().unwrap_or("");

        let seq = String::from_utf8(rec.seq().to_owned()).unwrap();
        fasta_map.insert(rec.id().to_string(), (desc.to_string(), seq));
    }

    Ok(fasta_map)
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
            // FIXME: should this be a hard error?
            eprintln!("Warning: sequence ID {} not found in FASTA", sequence.id());
        }
    }

    Ok(())
}

fn to_fasta(matches: &ArgMatches) -> ClstrResult<()> {
    let clstr_file = matches.get_one::<PathBuf>("FILE").unwrap().clone();
    let database_file = matches.get_one::<PathBuf>("DATABASE").unwrap().clone();

    // will this work for massive fastas..?
    let fasta_map = read_fasta(database_file)?;

    let cluster_parser = clstr::from_path(clstr_file.clone())?;

    for cluster in cluster_parser {
        let cluster = cluster?;

        let cluster_id =
            if let Some(representative_cluster_id) = cluster.get_representative().map(|e| e.id()) {
                let rcid = fasta_map
                    .get(representative_cluster_id)
                    .map(|(desc, _)| desc.clone())
                    .unwrap_or_else(|| "no-description".to_string());

                rcid.replace(" ", "_").replace("/", "_")
            } else {
                "No representative".to_string()
            };

        let out_file = File::create(clstr_file.with_extension(format!("{cluster_id}.fasta")))?;
        write_cluster_to_fasta(&cluster, &fasta_map, out_file)?;
    }

    Ok(())
}

fn stats(matches: &ArgMatches) -> ClstrResult<()> {
    let clstr_file = matches.get_one::<PathBuf>("FILE").unwrap().clone();
    let table = matches.get_flag("table");
    let parser = clstr::from_path(clstr_file.clone())?;

    // make a writer to stdout
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();

    if table {
        for cluster in parser {
            let cluster = cluster?;
            let _ = writeln!(handle, "{}\t{}", cluster.cluster_id(), cluster.size());
        }
        return Ok(());
    }

    let mut cluster_count = 0;
    let mut sequence_count = 0;

    for cluster in parser {
        let cluster = cluster?;
        cluster_count += 1;
        sequence_count += cluster.size();
    }

    let avg_sequence_count_per_cluster = sequence_count as f64 / cluster_count as f64;

    // write a tiny tsv
    let _ = writeln!(
        handle,
        "Cluster count\tSequence count\tAvg seqs per cluster"
    );
    let _ = writeln!(
        handle,
        "{cluster_count}\t{sequence_count}\t{avg_sequence_count_per_cluster}"
    );

    Ok(())
}

fn main() -> ClstrResult<()> {
    let matches = parse_args();

    let result = match matches.subcommand() {
        Some(("topn", matches)) => top_n(matches),
        Some(("tofasta", matches)) => to_fasta(matches),
        Some(("filtern", matches)) => filter_n(matches),
        Some(("stats", matches)) => stats(matches),
        _ => unreachable!("Exhausted list of subcommands and subcommand_required prevents `None`"),
    };

    if result.is_err() {
        eprintln!("clstr error: {}", result.unwrap_err());
    }

    Ok(())
}
