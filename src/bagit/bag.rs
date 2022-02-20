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
use log::{error, info, warn};
use regex::{Captures, Regex};
use snafu::ResultExt;
use walkdir::{DirEntry, WalkDir};

use crate::bagit::consts::*;
use crate::bagit::error::Error::*;
use crate::bagit::error::*;
use crate::bagit::tag::{
    read_bag_declaration, read_bag_info, write_bag_declaration, write_bag_info, BagDeclaration,
    BagInfo,
};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct BagItVersion {
    major: u8,
    minor: u8,
}

#[derive(Debug)]
pub struct Bag {
    base_dir: PathBuf,
    declaration: BagDeclaration,
    bag_info: BagInfo,
    algorithms: Vec<DigestAlgorithm>,
}

#[derive(Debug)]
pub struct BagUpdater {
    bag: Bag,
    recalculate_payload_manifests: bool,
}

#[derive(Debug)]
struct FileMeta {
    path: PathBuf,
    size_bytes: u64,
    digests: HashMap<DigestAlgorithm, HexDigest>,
}

// TODO investigate BagIt Profiles
// TODO note, when validating only unicode normalize if a file is not found
// TODO support 0.97
// TODO command for upgrading from 0.97 to 1.0?

// TODO update docs
/// Creates a new bag in place by moving the contents of `base_dir` into the bag's payload and
/// then writing all of the necessary tag files and manifests. The end result is that the `base_dir`
/// contains a fully formed bag.
///
/// The `algorithms` are the algorithms that are used when calculating file digests. If none are
/// provided, then `sha512` is used.
pub fn create_bag<S: AsRef<Path>, D: AsRef<Path>>(
    src_dir: S,
    dst_dir: D,
    mut bag_info: BagInfo,
    algorithms: &[DigestAlgorithm],
) -> Result<Bag> {
    let src_dir = src_dir.as_ref();
    let dst_dir = dst_dir.as_ref();

    info!("Creating bag in {}", dst_dir.display());

    let in_place = src_dir == dst_dir;
    let algorithms = defaulted_algorithms(algorithms);

    if !in_place {
        fs::create_dir_all(dst_dir).context(IoCreateSnafu { path: dst_dir })?;
    }

    let temp_name = format!("temp-{}", epoch_seconds());
    let temp_dir = dst_dir.join(&temp_name);

    fs::create_dir(&temp_dir).context(IoCreateSnafu { path: &temp_dir })?;

    let mut payload_meta = move_into_dir(!in_place, &src_dir, &temp_dir, &algorithms, |f| {
        f.file_name() != temp_name.as_str()
    })?;

    let data_dir = dst_dir.join(DATA);
    rename(temp_dir, &data_dir)?;

    add_data_prefix(&mut payload_meta);
    write_payload_manifests(&algorithms, &mut payload_meta, dst_dir)?;

    let declaration = BagDeclaration::new();
    write_bag_declaration(&declaration, dst_dir)?;

    if bag_info.bagging_date().is_none() {
        bag_info.add_bagging_date(current_date_str())?;
    }

    bag_info.add_software_agent(bagr_software_agent())?;
    bag_info.add_payload_oxum(build_payload_oxum(&payload_meta))?;

    write_bag_info(&bag_info, dst_dir)?;

    update_tag_manifests(dst_dir, &algorithms)?;

    Ok(Bag::new(dst_dir, declaration, bag_info, algorithms))
}

/// Opens a BagIt bag in that already exists in the specified directory
pub fn open_bag<P: AsRef<Path>>(base_dir: P) -> Result<Bag> {
    let base_dir = base_dir.as_ref();
    info!("Opening bag at {}", base_dir.display());

    let declaration = read_bag_declaration(base_dir)?;
    let algorithms = detect_digest_algorithms(base_dir)?;

    let bag_info = read_bag_info(base_dir)?;

    Ok(Bag::new(base_dir, declaration, bag_info, algorithms))
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

impl Bag {
    pub fn new<P: AsRef<Path>>(
        base_dir: P,
        declaration: BagDeclaration,
        bag_info: BagInfo,
        algorithms: Vec<DigestAlgorithm>,
    ) -> Self {
        Self {
            base_dir: base_dir.as_ref().into(),
            declaration,
            bag_info,
            algorithms,
        }
    }

    // TODO get tags
    // TODO get fetch entries
    // TODO download fetch entries

    /// Creates a `BagUpdater` that's used to update an existing bag
    pub fn update(self) -> BagUpdater {
        BagUpdater::new(self)
    }
}

impl BagUpdater {
    pub fn new(bag: Bag) -> Self {
        Self {
            bag,
            recalculate_payload_manifests: true,
        }
    }

    // TODO add algorithm
    // TODO modify tags
    // TODO add fetch item

    /// Enables/disables payload manifest recalculation on `finalize()`. This is enabled by default,
    /// but can be disabled if the digest algorithms in use have not changed and there were no
    /// changes to the payload.
    pub fn recalculate_payload_manifests(mut self, recalculate: bool) -> Self {
        self.recalculate_payload_manifests = recalculate;
        self
    }

    /// Writes the changes to disk and recalculates manifests.
    pub fn finalize(mut self) -> Result<Bag> {
        let base_dir = &self.bag.base_dir;
        let algorithms = &self.bag.algorithms;

        // TODO should updating these values keep the original position?
        // TODO need an option of overriding this
        self.bag.bag_info.add_bagging_date(current_date_str())?;
        self.bag.bag_info.add_software_agent(bagr_software_agent())?;

        if self.recalculate_payload_manifests {
            delete_payload_manifests(base_dir)?;
            let payload_meta = update_payload_manifests(base_dir, algorithms)?;
            self.bag
                .bag_info
                .add_payload_oxum(build_payload_oxum(&payload_meta))?;
        }

        write_bag_info(&self.bag.bag_info, base_dir)?;

        delete_tag_manifests(base_dir)?;
        update_tag_manifests(base_dir, algorithms)?;

        Ok(self.bag)
    }
}

/// Copies/moves the contents of the `src_dir` into the `dst_dir` and returns meta about all of the
/// moved files. If `copy_op` is true the files are copied, otherwise they're moved
fn move_into_dir<S, D, P>(
    copy_op: bool,
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

            let relative = file.path().strip_prefix(src_dir).unwrap();

            file_meta.push(FileMeta {
                path: relative.to_path_buf(),
                size_bytes: metadata.len(),
                digests: writer.finalize_hex(),
            });

            let file_dst = dst_dir.join(relative);

            fs::create_dir_all(file_dst.parent().unwrap())
                .context(IoCreateSnafu { path: &file_dst })?;

            if copy_op {
                copy(file.path(), file_dst)?;
            } else {
                rename(file.path(), file_dst)?;
            }
        } else if file.file_type().is_dir() {
            if !copy_op {
                dirs.push(file.path().to_path_buf());
            }
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

/// Calculates the digests for all of the payload files in the bag and writes the manifests
fn update_payload_manifests<P: AsRef<Path>>(
    base_dir: P,
    algorithms: &[DigestAlgorithm],
) -> Result<Vec<FileMeta>> {
    let base_dir = base_dir.as_ref();
    let mut meta = calculate_digests(base_dir.join(DATA), algorithms, |_| true)?;
    add_data_prefix(&mut meta);

    write_payload_manifests(algorithms, &mut meta, base_dir)?;

    Ok(meta)
}

/// Prefixes all payload files with `data/`
fn add_data_prefix(file_meta: &mut [FileMeta]) {
    let relative_data_dir = PathBuf::from(DATA);

    for meta in file_meta {
        meta.path = relative_data_dir.join(&meta.path);
    }
}

/// Calculates the digests for all of the tag files in the bag and writes the tag manifests
fn update_tag_manifests<P: AsRef<Path>>(base_dir: P, algorithms: &[DigestAlgorithm]) -> Result<()> {
    let base_dir = base_dir.as_ref();
    let mut meta = calculate_digests(base_dir, algorithms, |f| {
        // Skip the data directory and all tag manifests
        f.file_name() != DATA
            && f.file_name()
                .to_str()
                .map(|n| !TAG_MANIFEST_MATCHER.is_match(n))
                .unwrap_or(true)
    })?;
    write_tag_manifests(algorithms, &mut meta, base_dir)
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

fn write_payload_manifests<P: AsRef<Path>>(
    algorithms: &[DigestAlgorithm],
    file_meta: &mut [FileMeta],
    base_dir: P,
) -> Result<()> {
    // TODO this is currently not taking into account fetch.txt
    write_manifests(algorithms, file_meta, PAYLOAD_MANIFEST_PREFIX, base_dir)
}

fn write_tag_manifests<P: AsRef<Path>>(
    algorithms: &[DigestAlgorithm],
    file_meta: &mut [FileMeta],
    base_dir: P,
) -> Result<()> {
    write_manifests(algorithms, file_meta, TAG_MANIFEST_PREFIX, base_dir)
}

// TODO remember to consider * when reading
// TODO note when reading these files that `./data/` is ALLOWED
fn write_manifests<P: AsRef<Path>>(
    algorithms: &[DigestAlgorithm],
    file_meta: &mut [FileMeta],
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

    // Sort files so that they're written to the manifest deterministically
    file_meta.sort_by(|a, b| a.path.cmp(&b.path));

    for meta in file_meta {
        // TODO LF, CR, and % must be % encoded -- however! existing clients do NOT do this!
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
            writeln!(manifest, "{digest}  {path}").context(IoGeneralSnafu {})?;
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

fn copy<F: AsRef<Path>, T: AsRef<Path>>(from: F, to: T) -> Result<()> {
    let from = from.as_ref();
    let to = to.as_ref();
    info!("Copying {} to {}", from.display(), to.display());
    fs::copy(from, to)
        .map(|_| ())
        .context(IoCopySnafu { from, to })
}

/// Deletes all payload manifests in the base directory
fn delete_payload_manifests<P: AsRef<Path>>(base_dir: P) -> Result<()> {
    delete_matching_files(base_dir, &PAYLOAD_MANIFEST_MATCHER)
}

/// Deletes all tag manifests in the base directory
fn delete_tag_manifests<P: AsRef<Path>>(base_dir: P) -> Result<()> {
    delete_matching_files(base_dir, &TAG_MANIFEST_MATCHER)
}

fn delete_matching_files<P: AsRef<Path>>(base_dir: P, file_regex: &Regex) -> Result<()> {
    for_matching_files(base_dir, file_regex, |path, _| {
        info!("Deleting file {}", path.display());
        if let Err(e) = fs::remove_file(path) {
            if e.kind() != ErrorKind::NotFound {
                error!("Failed to delete file {}", path.display())
            }
        }
    })
}

fn detect_digest_algorithms<P: AsRef<Path>>(base_dir: P) -> Result<Vec<DigestAlgorithm>> {
    let mut algorithms = Vec::new();

    for_matching_files(base_dir, &PAYLOAD_MANIFEST_MATCHER, |_, captures| {
        let algorithm_str = captures.get(1).unwrap().as_str();
        match algorithm_str.try_into() {
            Ok(algorithm) => algorithms.push(algorithm),
            Err(_) => warn!("Detected unsupported digest algorithm: {algorithm_str}"),
        }
    })?;

    Ok(algorithms)
}

/// Iterates the files in a directory and applies `on_match` to the ones with file names that match
/// `file_regex`. `on_match` receives the path to the matched file as well as the captures from the
/// match.
fn for_matching_files<P, M>(base_dir: P, file_regex: &Regex, mut on_match: M) -> Result<()>
where
    P: AsRef<Path>,
    M: FnMut(&Path, &Captures),
{
    let base_dir = base_dir.as_ref();

    for file in fs::read_dir(base_dir).context(IoReadDirSnafu { path: base_dir })? {
        let file = file.context(IoReadDirSnafu { path: base_dir })?;
        if file
            .file_type()
            .context(IoStatSnafu { path: file.path() })?
            .is_file()
        {
            if let Some(file_name) = file.file_name().to_str() {
                if let Some(captures) = file_regex.captures(file_name) {
                    on_match(&file.path(), &captures);
                }
            }
        }
    }

    Ok(())
}

/// If the input is empty a new vec with the default algorithm is returned. Otherwise, the input
/// is deduped and a new vec is returned.
fn defaulted_algorithms(algorithms: &[DigestAlgorithm]) -> Vec<DigestAlgorithm> {
    if algorithms.is_empty() {
        vec![DEFAULT_ALGORITHM]
    } else {
        let mut new = Vec::from(algorithms);
        new.sort();
        new.dedup();
        new
    }
}

fn build_payload_oxum(file_meta: &[FileMeta]) -> String {
    let count = file_meta.len();
    let mut sum = 0;
    for meta in file_meta {
        sum += meta.size_bytes;
    }
    format!("{sum}.{count}")
}

fn bagr_software_agent() -> String {
    format!("bagr v{} <{}>", BAGR_VERSION, BAGR_SRC_URL)
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
