use snafu::prelude::*;
use std::path::PathBuf;

pub type Result<T, E = Error> = core::result::Result<T, E>;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("IO error: {}", source))]
    IoGeneral { source: std::io::Error },
    #[snafu(display("Error creating file {}: {}", path.display(), source))]
    IoCreate {
        source: std::io::Error,
        path: PathBuf,
    },
    #[snafu(display("Error writing to file {}: {}", path.display(), source))]
    IoWrite {
        source: std::io::Error,
        path: PathBuf,
    },
    #[snafu(display("Error reading file {}: {}", path.display(), source))]
    IoRead {
        source: std::io::Error,
        path: PathBuf,
    },
    #[snafu(display("Error reading directory {}: {}", path.display(), source))]
    IoReadDir {
        source: std::io::Error,
        path: PathBuf,
    },
    #[snafu(display("Failed to move {} to {}: {}", from.display(), to.display(), source))]
    IoMove {
        source: std::io::Error,
        from: PathBuf,
        to: PathBuf,
    },
    #[snafu(display("Error walking files: {}", source))]
    WalkFile { source: walkdir::Error },
}
