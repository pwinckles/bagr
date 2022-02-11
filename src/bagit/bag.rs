use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};

use crate::bagit::error::Result;
use crate::bagit::tag::{Tag, TagFileWriter};

// TODO move?
pub const BAGIT_1_0: BagItVersion = BagItVersion::new(1, 0);
pub const BAGIT_DEFAULT_VERSION: BagItVersion = BAGIT_1_0;

// Filenames
pub const BAGIT_TXT: &str = "bagit.txt";

// Bag declaration tag labels
pub const LABEL_BAGIT_VERSION: &str = "BagIt-Version";
pub const LABEL_FILE_ENCODING: &str = "Tag-File-Character-Encoding";

#[derive(Debug)]
pub struct Bag {
    base_dir: PathBuf,
    declaration: BagDeclaration,
}

/// Builder for constructing new bags
#[derive(Debug)]
pub struct BagBuilder {
    /// The directory to create the bag in
    base_dir: PathBuf,
    /// Paths that should be recursively copied into bag's payload
    copy_paths: Vec<PathBuf>,
    /// Paths that should be moved into the bag's payload
    move_paths: Vec<PathBuf>,
}

// TODO need to string
// TODO need comparator
#[derive(Debug, Copy, Clone)]
pub struct BagItVersion {
    major: u8,
    minor: u8,
}

#[derive(Debug)]
pub struct BagDeclaration {
    version: BagItVersion,
    // TODO figure out how to handle non-utf-8 encodings
    // https://crates.io/crates/encoding_rs
    // https://crates.io/crates/encoding_rs_io
    // Encoding will always be UTF-8 when creating, but it could be different when reading
    encoding: String,
}

impl Bag {
    pub fn new<P: AsRef<Path>>(base_dir: P, declaration: BagDeclaration) -> Self {
        Self {
            base_dir: base_dir.as_ref().into(),
            declaration,
        }
    }
}

impl BagBuilder {
    /// Creates a new `BagBuilder`. The bag will be built at the specified base directory, which
    /// *must* already exist.
    pub fn new<P: AsRef<Path>>(base_dir: P) -> Self {
        Self {
            base_dir: base_dir.as_ref().into(),
            copy_paths: Vec::new(),
            move_paths: Vec::new(),
        }
    }

    /// Adds a path that should be recursively copied into the bag's payload
    pub fn copy_files<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.copy_paths.push(path.as_ref().into());
        self
    }

    /// Adds a path that should be moved into the bag's payload
    pub fn move_files<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.move_paths.push(path.as_ref().into());
        self
    }

    // TODO this should be a result -- need to figure out snafu
    /// Constructs the bag
    pub fn build(self) -> Result<Bag> {
        let declaration = BagDeclaration::new();

        // TODO move
        let tag_writer = TagFileWriter::new();

        tag_writer.write(&declaration.as_tags(), self.base_dir.join(BAGIT_TXT))?;

        // TODO create bagit.txt
        // TODO create data/
        // TODO copy files
        // TODO move files
        // TODO write payload manifest
        // TODO write tag manifest

        Ok(Bag::new(self.base_dir, declaration))
    }
}

impl BagItVersion {
    pub const fn new(major: u8, minor: u8) -> Self {
        Self {
            major,
            minor,
        }
    }
}

impl Display for BagItVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

// TODO add method for creating from tag array
// TODO add method for converting to a tag array
impl BagDeclaration {
    pub fn new() -> Self {
        Self {
            version: BAGIT_DEFAULT_VERSION,
            // TODO encoding
            encoding: "UTF-8".into(),
        }
    }

    pub fn with_values<S: AsRef<str>>(version: BagItVersion, encoding: S) -> Self {
        Self {
            version,
            encoding: encoding.as_ref().into(),
        }
    }

    pub fn as_tags(&self) -> Vec<Tag> {
        vec![
            Tag::with_value(LABEL_BAGIT_VERSION, self.version.to_string()),
            Tag::with_value(LABEL_FILE_ENCODING, &self.encoding),
        ]
    }
}

impl Default for BagDeclaration {
    fn default() -> Self {
        Self::new()
    }
}
