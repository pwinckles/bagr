use crate::bagit::bag::BagItVersion;
use snafu::prelude::*;
use std::path::PathBuf;
use std::string::FromUtf8Error;

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
    #[snafu(display("Failed to copy {} to {}: {}", from.display(), to.display(), source))]
    IoCopy {
        source: std::io::Error,
        from: PathBuf,
        to: PathBuf,
    },
    #[snafu(display("Failed to delete {}: {}", path.display(), source))]
    IoDelete {
        source: std::io::Error,
        path: PathBuf,
    },
    #[snafu(display("Failed to stat {}: {}", path.display(), source))]
    IoStat {
        source: std::io::Error,
        path: PathBuf,
    },
    #[snafu(display("Error walking files: {}", source))]
    WalkFile { source: walkdir::Error },
    #[snafu(display("Encountered an unsupported file type at {}", path.display()))]
    UnsupportedFile { path: PathBuf },
    #[snafu(display("Invalid tag line: {details}"))]
    InvalidTagLine { details: String },
    #[snafu(display("Tag number {num} in file {} is invalid: {details}", path.display()))]
    InvalidTagLineWithRef {
        path: PathBuf,
        num: u32,
        details: String,
    },
    #[snafu(display("Invalid tag with label '{label}': {details}"))]
    InvalidTag { label: String, details: String },
    #[snafu(display("Invalid BagIt version: {value}"))]
    InvalidBagItVersion { value: String },
    #[snafu(display("Missing required tag {tag}"))]
    MissingTag { tag: String },
    #[snafu(display("Unsupported BagIt version {version}"))]
    UnsupportedVersion { version: BagItVersion },
    #[snafu(display("Unsupported file encoding {encoding}"))]
    UnsupportedEncoding { encoding: String },
    #[snafu(display("Failed to decode string: {source}"))]
    InvalidString { source: FromUtf8Error },
}
