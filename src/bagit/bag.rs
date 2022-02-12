use chrono::Local;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::Write;
use std::io::{BufWriter, ErrorKind};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{fs, io};

use crate::bagit::digest::{DigestAlgorithm, HexDigest, MultiDigestWriter};
use log::info;
use snafu::ResultExt;
use walkdir::{DirEntry, WalkDir};

use crate::bagit::consts::*;
use crate::bagit::error::Error::*;
use crate::bagit::error::*;
use crate::bagit::tag::{read_bag_declaration, write_tag_file, BagDeclaration, BagInfo};

#[derive(Debug)]
pub struct Bag {
    base_dir: PathBuf,
    declaration: BagDeclaration,
}

// TODO need to string
// TODO need comparator
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct BagItVersion {
    major: u8,
    minor: u8,
}

#[derive(Debug)]
struct FileMeta {
    path: PathBuf,
    size_bytes: u64,
    digests: HashMap<DigestAlgorithm, HexDigest>,
}

/// Creates a new bag in place by moving the contents of `base_dir` into the bag's payload and
/// then writing all of the necessary tag files and manifests. The end result is that the `base_dir`
/// contains a fully formed bag.
///
/// The `algorithms` are the algorithms that are used when calculating file digests. If none are
/// provided, then `sha512` is used.
pub fn create_bag<P: AsRef<Path>>(base_dir: P, algorithms: &[DigestAlgorithm]) -> Result<Bag> {
    // TODO ctrl+c wiring

    let base_dir = base_dir.as_ref();
    let algorithms = if algorithms.is_empty() {
        &[DigestAlgorithm::Sha512]
    } else {
        algorithms
    };

    let temp_name = format!("temp-{}", epoch_seconds());
    let temp_dir = base_dir.join(&temp_name);

    fs::create_dir(&temp_dir).context(IoCreateSnafu { path: &temp_dir })?;

    let mut payload_meta = move_into_dir(&base_dir, &temp_dir, algorithms, |f| {
        f.file_name() != temp_name.as_str()
    })?;

    let relative_data_dir = PathBuf::from(DATA);

    for meta in &mut payload_meta {
        meta.path = relative_data_dir.join(&meta.path);
    }

    let data_dir = base_dir.join(DATA);
    rename(temp_dir, &data_dir)?;

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
                // TODO this would be better as a regex match
                .map(|n| !n.starts_with(TAG_MANIFEST_PREFIX))
                .unwrap_or(true)
    })?;

    write_manifests(algorithms, &tag_meta, TAG_MANIFEST_PREFIX, base_dir)?;

    Ok(Bag::new(base_dir, declaration))
}

// TODO docs
pub fn open_bag<P: AsRef<Path>>(base_dir: P) -> Result<Bag> {
    let base_dir = base_dir.as_ref();
    info!("Opening bag at {}", base_dir.display());
    let declaration = read_bag_declaration(base_dir)?;
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

impl TryFrom<String> for BagItVersion {
    type Error = Error;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        TryFrom::try_from(&value)
    }
}

impl TryFrom<&String> for BagItVersion {
    type Error = Error;

    fn try_from(value: &String) -> std::result::Result<Self, Self::Error> {
        if let Some((major, minor)) = value.split_once('.') {
            let major = major.parse::<u8>().map_err(|_| InvalidBagItVersion {
                value: value.into(),
            })?;
            let minor = minor.parse::<u8>().map_err(|_| InvalidBagItVersion {
                value: value.into(),
            })?;
            Ok(BagItVersion::new(major, minor))
        } else {
            Err(InvalidBagItVersion {
                value: value.into(),
            })
        }
    }
}

/// Moves the contents of the `src_dir` into the `dst_dir` and returns meta about all of the
/// moved files.
fn move_into_dir<S, D, P>(
    src_dir: S,
    dst_dir: D,
    algorithms: &[DigestAlgorithm],
    predicate: P,
) -> Result<Vec<FileMeta>>
where
    S: AsRef<Path>,
    D: AsRef<Path>,
    P: FnMut(&DirEntry) -> bool,
{
    let src_dir = src_dir.as_ref();
    let dst_dir = dst_dir.as_ref();

    let mut file_meta = Vec::new();
    let mut dirs = Vec::new();

    for file in WalkDir::new(src_dir).into_iter().filter_entry(predicate) {
        let file = file.context(WalkFileSnafu {})?;

        if file.file_type().is_file() {
            let metadata = file.metadata().context(WalkFileSnafu {})?;

            info!("Calculating digests for {}", file.path().display());

            let mut writer = MultiDigestWriter::new(algorithms, std::io::sink());
            let mut reader = File::open(file.path()).context(IoReadSnafu { path: file.path() })?;

            io::copy(&mut reader, &mut writer).context(IoReadSnafu { path: file.path() })?;

            file_meta.push(FileMeta {
                path: file.path().strip_prefix(src_dir).unwrap().to_path_buf(),
                size_bytes: metadata.len(),
                digests: writer.finalize_hex(),
            });

            let relative = file.path().strip_prefix(src_dir).unwrap();
            let file_dst = dst_dir.join(relative);

            fs::create_dir_all(file_dst.parent().unwrap())
                .context(IoCreateSnafu { path: &file_dst })?;
            rename(file.path(), file_dst)?;
        } else if file.file_type().is_dir() {
            dirs.push(file.path().to_path_buf());
        } else {
            return Err(UnsupportedFile {
                path: file.path().to_path_buf(),
            });
        }
    }

    // Delete any dangling directories left after moving out all of the files
    for dir in dirs {
        if dir == src_dir {
            continue;
        }
        if let Err(e) = fs::remove_dir_all(&dir) {
            if e.kind() != ErrorKind::NotFound {
                return Err(IoDelete {
                    path: dir,
                    source: e,
                });
            }
        }
    }

    Ok(file_meta)
}

/// Calculates the digests for all of the files under the `base_dir`
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
            let metadata = file.metadata().context(WalkFileSnafu {})?;

            info!("Calculating digests for {}", file.path().display());

            let mut writer = MultiDigestWriter::new(algorithms, std::io::sink());
            let mut reader = File::open(file.path()).context(IoReadSnafu { path: file.path() })?;

            io::copy(&mut reader, &mut writer).context(IoReadSnafu { path: file.path() })?;

            file_meta.push(FileMeta {
                path: file.path().strip_prefix(base_dir).unwrap().to_path_buf(),
                size_bytes: metadata.len(),
                digests: writer.finalize_hex(),
            });
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
        // TODO on windows, `\` must be converted to `/`
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
