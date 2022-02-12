use std::ffi::OsStr;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use log::info;
use snafu::ResultExt;

use crate::bagit::error::*;
use crate::bagit::tag::{TagFileWriter, TagList};

// TODO move?
pub const BAGIT_1_0: BagItVersion = BagItVersion::new(1, 0);
pub const BAGIT_DEFAULT_VERSION: BagItVersion = BAGIT_1_0;

// Filenames
pub const BAGIT_TXT: &str = "bagit.txt";
pub const BAG_INFO_TXT: &str = "bag-info.txt";
pub const FETCH_TXT: &str = "fetch.txt";
pub const DATA: &str = "data";

// Bag declaration tag labels
pub const LABEL_BAGIT_VERSION: &str = "BagIt-Version";
pub const LABEL_FILE_ENCODING: &str = "Tag-File-Character-Encoding";

#[derive(Debug)]
pub struct Bag {
    base_dir: PathBuf,
    declaration: BagDeclaration,
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

pub fn create_bag<P: AsRef<Path>>(base_dir: P) -> Result<Bag> {
    let base_dir = base_dir.as_ref();

    let temp_name = format!("temp-{}", epoch_seconds());
    let temp_dir = base_dir.join(&temp_name);

    fs::create_dir(&temp_dir).context(IoCreateSnafu {
        path: &temp_dir
    })?;

    let base_files = fs::read_dir(base_dir).context(IoReadDirSnafu {
        path: base_dir
    })?;

    for file in base_files {
        let file = file.context(IoGeneralSnafu {})?;
        let file_name = file.file_name();
        if <String as AsRef<OsStr>>::as_ref(&temp_name) != file_name {
            rename(file.path(), temp_dir.join(file_name))?;
        }
    }

    let data_dir = base_dir.join(DATA);

    rename(temp_dir, &data_dir)?;

    let declaration = BagDeclaration::new();
    // TODO move
    let tag_writer = TagFileWriter::new();
    tag_writer.write(&declaration.to_tags(), base_dir.join(BAGIT_TXT))?;

    // TODO calculate digests
    // TODO write payload manifests
    // TODO write bagit.txt
    // TODO write bag-info.txt
    // TODO calculate tag digests
    // TODO write tag manifests

    Ok(Bag::new(base_dir, declaration))
}

impl Bag {
    pub fn new<P: AsRef<Path>>(base_dir: P, declaration: BagDeclaration) -> Self {
        Self {
            base_dir: base_dir.as_ref().into(),
            declaration,
        }
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

    pub fn to_tags(&self) -> TagList {
        let mut tags = TagList::with_capacity(2);
        tags.add_tag(LABEL_BAGIT_VERSION, self.version.to_string());
        tags.add_tag(LABEL_FILE_ENCODING, &self.encoding);
        tags
    }
}

impl Default for BagDeclaration {
    fn default() -> Self {
        Self::new()
    }
}

fn epoch_seconds() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH)
        .expect("Failed to get system time").as_secs()
}

fn rename<F: AsRef<Path>, T: AsRef<Path>>(from: F, to: T) -> Result<()> {
    let from = from.as_ref();
    let to = to.as_ref();
    info!("Moving {} to {}", from.display(), to.display());
    fs::rename(from, to).context(IoMoveSnafu {
        from,
        to,
    })
}
