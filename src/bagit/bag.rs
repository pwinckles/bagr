use chrono::Local;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{fs, io};

use crate::bagit::digest::{DigestAlgorithm, HexDigest, MultiDigestWriter};
use log::info;
use snafu::ResultExt;
use walkdir::{DirEntry, WalkDir};

use crate::bagit::error::*;
use crate::bagit::tag::{write_tag_file, TagList};

// TODO move?
pub const BAGIT_1_0: BagItVersion = BagItVersion::new(1, 0);
pub const BAGIT_DEFAULT_VERSION: BagItVersion = BAGIT_1_0;

// Filenames
pub const BAGIT_TXT: &str = "bagit.txt";
pub const BAG_INFO_TXT: &str = "bag-info.txt";
pub const FETCH_TXT: &str = "fetch.txt";
pub const DATA: &str = "data";
pub const PAYLOAD_MANIFEST_PREFIX: &str = "manifest";
pub const TAG_MANIFEST_PREFIX: &str = "tagmanifest";

// bagit.txt tag labels
pub const LABEL_BAGIT_VERSION: &str = "BagIt-Version";
pub const LABEL_FILE_ENCODING: &str = "Tag-File-Character-Encoding";

// bag-info.txt reserved labels
pub const LABEL_BAGGING_DATE: &str = "Bagging-Date";
pub const LABEL_PAYLOAD_OXUM: &str = "Payload-Oxum";

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

#[derive(Debug)]
pub struct BagInfo {
    tags: TagList,
}

#[derive(Debug)]
struct FileMeta {
    path: PathBuf,
    size_bytes: u64,
    digests: HashMap<DigestAlgorithm, HexDigest>,
}

pub fn create_bag<P: AsRef<Path>>(base_dir: P, algorithms: &[DigestAlgorithm]) -> Result<Bag> {
    // TODO ctrl+c wiring

    let base_dir = base_dir.as_ref();

    let temp_name = format!("temp-{}", epoch_seconds());
    let temp_dir = base_dir.join(&temp_name);

    fs::create_dir(&temp_dir).context(IoCreateSnafu { path: &temp_dir })?;

    let base_files = fs::read_dir(base_dir).context(IoReadDirSnafu { path: base_dir })?;

    for file in base_files {
        let file = file.context(IoGeneralSnafu {})?;
        let file_name = file.file_name();

        if <String as AsRef<OsStr>>::as_ref(&temp_name) != file_name {
            // TODO this is not correct because it will move symlinks
            //      need to walk the files, move only files, and delete the rest
            //      calculating digests at this time would make sense so we only walk once
            rename(file.path(), temp_dir.join(file_name))?;
        }
    }

    let data_dir = base_dir.join(DATA);
    rename(temp_dir, &data_dir)?;

    let mut payload_meta = calculate_digests(&data_dir, algorithms, |_| true)?;
    let relative_data_dir = PathBuf::from(DATA);

    for meta in &mut payload_meta {
        meta.path = relative_data_dir.join(&meta.path);
    }

    write_manifests(algorithms, &payload_meta, PAYLOAD_MANIFEST_PREFIX, base_dir)?;

    let declaration = BagDeclaration::new();
    write_tag_file(&declaration.to_tags(), base_dir.join(BAGIT_TXT))?;

    let mut bag_info = BagInfo::with_capacity(2);
    bag_info.add_bagging_date(current_date_str());
    bag_info.add_payload_oxum(build_payload_oxum(&payload_meta));

    write_tag_file(&bag_info.into(), base_dir.join(BAG_INFO_TXT))?;

    let tag_meta = calculate_digests(base_dir, algorithms, |f| {
        // Skip the data directory and all tag manifests
        f.file_name() != DATA
            && f.file_name()
                .to_str()
                .map(|n| !n.starts_with(TAG_MANIFEST_PREFIX))
                .unwrap_or(true)
    })?;

    write_manifests(algorithms, &tag_meta, TAG_MANIFEST_PREFIX, base_dir)?;

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
        Self { major, minor }
    }
}

impl Display for BagItVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

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

// TODO From<> for TagList -> BagDeclaration

impl BagInfo {
    pub fn new() -> Self {
        Self {
            tags: TagList::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            tags: TagList::with_capacity(capacity),
        }
    }

    pub fn with_tags(tags: TagList) -> Self {
        Self { tags }
    }

    pub fn add_bagging_date<S: AsRef<str>>(&mut self, value: S) -> &mut Self {
        self.tags.add_tag(LABEL_BAGGING_DATE, value);
        self
    }

    pub fn add_payload_oxum<S: AsRef<str>>(&mut self, value: S) -> &mut Self {
        self.tags.remove_tags(LABEL_PAYLOAD_OXUM);
        self.tags.add_tag(LABEL_PAYLOAD_OXUM, value);
        self
    }
}

impl Default for BagInfo {
    fn default() -> Self {
        Self::new()
    }
}

impl From<TagList> for BagInfo {
    fn from(tags: TagList) -> Self {
        BagInfo::with_tags(tags)
    }
}

impl From<BagInfo> for TagList {
    fn from(info: BagInfo) -> Self {
        info.tags
    }
}

fn calculate_digests<D, P>(
    base_dir: D,
    algorithms: &[DigestAlgorithm],
    predicate: P,
) -> Result<Vec<FileMeta>>
where
    D: AsRef<Path>,
    P: FnMut(&DirEntry) -> bool,
{
    let base_dir = base_dir.as_ref();
    let mut file_meta = Vec::new();

    for file in WalkDir::new(base_dir).into_iter().filter_entry(predicate) {
        let file = file.context(WalkFileSnafu {})?;

        if file.file_type().is_file() {
            // TODO there's a question if we need this here
            let metadata = file.metadata().context(WalkFileSnafu {})?;

            info!("Calculating digests for {}", file.path().display());

            let mut writer = MultiDigestWriter::new(algorithms, std::io::sink());
            let mut reader = File::open(file.path()).context(IoReadSnafu { path: file.path() })?;

            io::copy(&mut reader, &mut writer).context(IoReadSnafu { path: file.path() })?;

            file_meta.push(FileMeta {
                path: file.path().strip_prefix(base_dir).unwrap().to_path_buf(),
                size_bytes: metadata.len(),
                digests: writer.finalize_hex(),
            })
        }
    }

    Ok(file_meta)
}

fn write_manifests<P: AsRef<Path>>(
    algorithms: &[DigestAlgorithm],
    file_meta: &[FileMeta],
    prefix: &str,
    base_dir: P,
) -> Result<()> {
    let base_dir = base_dir.as_ref();

    let mut manifests = HashMap::with_capacity(algorithms.len());

    for algorithm in algorithms {
        let manifest = base_dir.join(format!("{prefix}-{algorithm}.txt"));
        info!("Writing manifest {}", manifest.display());
        let file = File::create(&manifest).context(IoCreateSnafu { path: manifest })?;
        manifests.insert(algorithm, BufWriter::new(file));
    }

    for meta in file_meta {
        // TODO LF and CR must be % encoded
        let path = meta.path.display();
        for algorithm in algorithms {
            let digest = meta
                .digests
                .get(algorithm)
                .expect("Missing expected file digest");
            let manifest = manifests
                .get_mut(algorithm)
                .expect("Missing expected file digest");
            writeln!(manifest, "{digest} {path}").context(IoGeneralSnafu {})?;
        }
    }

    Ok(())
}

fn rename<F: AsRef<Path>, T: AsRef<Path>>(from: F, to: T) -> Result<()> {
    let from = from.as_ref();
    let to = to.as_ref();
    info!("Moving {} to {}", from.display(), to.display());
    fs::rename(from, to).context(IoMoveSnafu { from, to })
}

fn build_payload_oxum(file_meta: &[FileMeta]) -> String {
    let count = file_meta.len();
    let mut sum = 0;
    for meta in file_meta {
        sum += meta.size_bytes;
    }
    format!("{sum}.{count}")
}

fn current_date_str() -> String {
    Local::today().format("%Y-%m-%d").to_string()
}

fn epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Failed to get system time")
        .as_secs()
}
