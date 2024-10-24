/*!
A small crate to parse CD-HIT's .clstr file format. *Only tested with CD-HIT, not CD-HIT-EST.*
Or actually another program in the `cd-hit` suite.
*/

use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::num::{ParseFloatError, ParseIntError};
use std::path::Path;

/// A type alias for `Result<T, clstr::Error>`.
pub type Result<T> = std::result::Result<T, Error>;

/// An error type for this crate.
#[derive(Debug)]
pub struct Error(Box<ErrorKind>);

impl Error {
    pub(crate) fn new(kind: ErrorKind) -> Error {
        Error(Box::new(kind))
    }

    pub fn kind(&self) -> &ErrorKind {
        &self.0
    }

    pub fn into_kind(self) -> ErrorKind {
        *self.0
    }
}

#[derive(Debug)]
pub enum ErrorKind {
    Io(io::Error),
    Int(ParseIntError),
    Float(ParseFloatError),
    ReadRecord(String),
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::new(ErrorKind::Io(err))
    }
}

impl From<ParseIntError> for Error {
    fn from(err: ParseIntError) -> Self {
        Error::new(ErrorKind::Int(err))
    }
}

impl From<ParseFloatError> for Error {
    fn from(err: ParseFloatError) -> Self {
        Error::new(ErrorKind::Float(err))
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self.0 {
            ErrorKind::Io(ref err) => write!(f, "I/O error - {}", err),
            ErrorKind::Int(ref err) => write!(f, "parsing integer error - {}", err),
            ErrorKind::Float(ref err) => write!(f, "parsing float error - {}", err),
            ErrorKind::ReadRecord(ref err) => write!(f, "reading record - {}", err),
        }
    }
}

impl std::error::Error for Error {}

/// Represents a single sequence entry in a cluster.
#[derive(Debug)]
pub struct Sequence {
    /// The length of the sequence.
    length: u32,
    /// The sequence ID.
    id: String,
    /// The percentage identity to the representative sequence.
    identity: Option<f32>,
    /// Whether this sequence is the representative sequence.
    is_representative: bool,
}

impl Sequence {
    /// Returns the length of the sequence.
    pub fn length(&self) -> u32 {
        self.length
    }

    /// Returns the sequence ID.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the percentage identity to the representative sequence, if available.
    pub fn identity(&self) -> Option<f32> {
        self.identity
    }

    /// Returns whether this sequence is the representative sequence.
    pub fn is_representative(&self) -> bool {
        self.is_representative
    }
}

/// Represents a cluster containing multiple sequences.
#[derive(Debug)]
pub struct Cluster {
    /// The cluster ID.
    cluster_id: usize,
    /// The sequences in this cluster.
    sequences: Vec<Sequence>,
}

impl Cluster {
    /// Returns the cluster ID.
    pub fn cluster_id(&self) -> usize {
        self.cluster_id
    }

    /// Returns the sequences in this cluster.
    pub fn sequences(&self) -> &Vec<Sequence> {
        &self.sequences
    }

    /// Returns the representative sequence, if available.
    pub fn get_representative(&self) -> Option<&Sequence> {
        self.sequences.iter().find(|s| s.is_representative)
    }

    /// Returns the number of sequences in this cluster.
    pub fn size(&self) -> usize {
        self.sequences.len()
    }
}

/// Iterator to parse `.clstr` file.
pub struct ClstrParser<R: BufRead> {
    /// The reader to parse the file.
    reader: R,
    /// The current cluster being parsed.
    current_cluster: Option<Cluster>,
}

impl<R: BufRead> ClstrParser<R> {
    pub fn new(reader: R) -> Self {
        ClstrParser {
            reader,
            current_cluster: None,
        }
    }
}

impl<R: BufRead> Iterator for ClstrParser<R> {
    type Item = Result<Cluster>;

    fn next(&mut self) -> Option<Self::Item> {
        for line_result in self.reader.by_ref().lines() {
            let line = match line_result {
                Ok(l) => l,
                Err(e) => return Some(Err(Error::from(e))),
            };

            if line.starts_with('>') {
                if let Some(c) = self.current_cluster.take() {
                    self.current_cluster = Some(Cluster {
                        cluster_id: c.cluster_id + 1,
                        sequences: Vec::new(),
                    });
                    return Some(Ok(c));
                }

                self.current_cluster = Some(Cluster {
                    cluster_id: self
                        .current_cluster
                        .as_ref()
                        .map_or(0, |c| c.cluster_id + 1),
                    sequences: Vec::new(),
                });
            } else if let Some(ref mut c) = self.current_cluster {
                match parse_sequence_line(&line) {
                    Ok(seq) => c.sequences.push(seq),
                    Err(e) => return Some(Err(e)),
                }
            }
        }

        self.current_cluster.take().map(Ok)
    }
}

/// Parse a single sequence line from a cluster file.
fn parse_sequence_line(line: &str) -> Result<Sequence> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 3 {
        return Err(Error::new(ErrorKind::ReadRecord(format!(
            "Invalid sequence line: {}",
            line
        ))));
    }

    let length_string = parts[1].to_string();
    let length = length_string
        // FIXME: this only works for amino acids
        .strip_suffix("aa,")
        .ok_or_else(|| {
            Error::new(ErrorKind::ReadRecord(format!(
                "Invalid length format: {}",
                line
            )))
        })?
        .parse::<u32>()
        .map_err(Error::from)?;

    let id = parts[2]
        .trim_start_matches('>')
        .split("...")
        .next()
        .ok_or_else(|| {
            Error::new(ErrorKind::ReadRecord(format!(
                "Invalid ID format: {}",
                line
            )))
        })?
        .to_string();

    let is_representative = line.ends_with('*');

    let identity = if let Some(at_pos) = line.find(" at ") {
        Some(
            line[at_pos + 4..]
                .trim_end_matches('%')
                .parse::<f32>()
                .map_err(Error::from)?,
        )
    } else {
        None
    };

    Ok(Sequence {
        length,
        id,
        identity,
        is_representative,
    })
}

/// Function to parse a `.clstr` file from a path.
pub fn from_path<P: AsRef<Path>>(path: P) -> Result<ClstrParser<BufReader<File>>> {
    let file = File::open(path).map_err(Error::from)?;
    let reader = BufReader::new(file);
    Ok(ClstrParser::new(reader))
}

/// Function to parse a `.clstr` file from a reader.
pub fn from_reader<R: BufRead>(reader: R) -> ClstrParser<R> {
    ClstrParser::new(reader)
}

/// Struct to write `.clstr` format files.
pub struct ClstrWriter<W: Write> {
    writer: W,
}

impl<W: Write> ClstrWriter<W> {
    /// Creates a new `ClstrWriter`.
    pub fn new(writer: W) -> Self {
        ClstrWriter { writer }
    }

    /// Writes a cluster to the `.clstr` format.
    pub fn write_cluster(&mut self, cluster: &Cluster) -> Result<()> {
        // Write the cluster header: >Cluster <ID>
        writeln!(self.writer, ">Cluster {}", cluster.cluster_id())?;

        // Write each sequence in the cluster.
        for (index, seq) in cluster.sequences().iter().enumerate() {
            self.write_sequence(index, seq)?;
        }

        Ok(())
    }

    /// Writes a single sequence to the `.clstr` format.
    fn write_sequence(&mut self, index: usize, sequence: &Sequence) -> Result<()> {
        // Format sequence like: 0    4481aa, >sp|P0C6T5|R1A_BCHK5... at 99.89%
        write!(
            self.writer,
            "{}    {}aa, >{}...",
            index,
            sequence.length(),
            sequence.id()
        )?;

        // If there's an identity percentage, write it
        if let Some(identity) = sequence.identity() {
            write!(self.writer, " at {:.2}%", identity)?;
        }

        // Mark the representative sequence with an asterisk (*)
        if sequence.is_representative() {
            write!(self.writer, " *")?;
        }

        writeln!(self.writer)?;

        Ok(())
    }

    /// Finalize the writer by flushing any remaining output.
    pub fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

/// Helper function to create a writer from a file path.
pub fn to_path<P: AsRef<Path>>(path: P) -> Result<ClstrWriter<File>> {
    let file = File::create(path)?;
    Ok(ClstrWriter::new(file))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_clstr_parsing() {
        let data = b">Cluster 0
0    4481aa, >sp|P0C6T5|R1A_BCHK5... at 99.89%
1    7126aa, >sp|P0C6W1|R1AB_BC133... at 66.94%
2    7119aa, >sp|P0C6W3|R1AB_BCHK4... at 67.17%
3    7182aa, >sp|P0C6W4|R1AB_BCHK5... *
4    307aa, >sp|Q9WQ77|R1AB_CVRSD... at 76.22%
>Cluster 1
0    4471aa, >sp|P0C6U3|R1A_CVHN1... at 99.91%
1    4441aa, >sp|P0C6U4|R1A_CVHN2... at 81.47%
2    4421aa, >sp|P0C6U5|R1A_CVHN5... at 81.52%
" as &[u8];

        let mut parser = ClstrParser::new(data);

        let cluster0 = parser.next().unwrap().unwrap();
        assert_eq!(cluster0.cluster_id(), 0);
        assert_eq!(cluster0.size(), 5);
        assert_eq!(cluster0.sequences()[0].id(), "sp|P0C6T5|R1A_BCHK5");
        assert_eq!(cluster0.sequences()[0].identity(), Some(99.89));
        assert!(!cluster0.sequences()[0].is_representative());

        assert_eq!(cluster0.sequences()[3].id(), "sp|P0C6W4|R1AB_BCHK5");
        assert!(cluster0.sequences()[3].is_representative());
        assert_eq!(cluster0.sequences()[3].identity(), None);

        let cluster1 = parser.next().unwrap().unwrap();
        assert_eq!(cluster1.cluster_id(), 1);
        assert_eq!(cluster1.size(), 3);
        assert_eq!(cluster1.sequences()[0].id(), "sp|P0C6U3|R1A_CVHN1");
        assert_eq!(cluster1.sequences()[0].identity(), Some(99.91));
        assert!(!cluster1.sequences()[0].is_representative());
    }

    #[test]
    fn test_write_cluster() {
        let sequence1 = Sequence {
            length: 4481,
            id: "sp|P0C6T5|R1A_BCHK5".to_string(),
            identity: Some(99.89),
            is_representative: false,
        };

        let sequence2 = Sequence {
            length: 7182,
            id: "sp|P0C6W4|R1AB_BCHK5".to_string(),
            identity: None,
            is_representative: true,
        };

        let cluster = Cluster {
            cluster_id: 0,
            sequences: vec![sequence1, sequence2],
        };

        let mut output = Cursor::new(Vec::new());
        let mut writer = ClstrWriter::new(&mut output);
        writer.write_cluster(&cluster).unwrap();
        writer.flush().unwrap();

        let output_str = String::from_utf8(output.into_inner()).unwrap();
        assert_eq!(output_str, ">Cluster 0\n0    4481aa, >sp|P0C6T5|R1A_BCHK5... at 99.89%\n1    7182aa, >sp|P0C6W4|R1AB_BCHK5... *\n");
    }
}
