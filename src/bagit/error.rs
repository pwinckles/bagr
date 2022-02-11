use std::path::PathBuf;
use snafu::prelude::*;

pub type Result<T, E = Error> = core::result::Result<T, E>;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
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
}
