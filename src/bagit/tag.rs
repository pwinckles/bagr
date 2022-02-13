use log::{debug, info};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;
use std::slice::Iter;
use std::vec::IntoIter;

use snafu::ResultExt;

use crate::bagit::bag::BagItVersion;
use crate::bagit::consts::*;
use crate::bagit::error::*;
use crate::bagit::Error::{InvalidTagLine, MissingTag, UnsupportedEncoding, UnsupportedVersion};

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
pub struct Tag {
    label: String,
    value: String,
}

#[derive(Debug)]
pub struct TagList {
    tags: Vec<Tag>,
}

/// Writes bagit.txt to the bag's base directory
pub fn write_bag_declaration<P: AsRef<Path>>(
    bag_declaration: &BagDeclaration,
    base_dir: P,
) -> Result<()> {
    write_tag_file(
        &bag_declaration.to_tags(),
        base_dir.as_ref().join(BAGIT_TXT),
    )
}

/// Writes bag-info.txt to the bag's base directory
pub fn write_bag_info<P: AsRef<Path>>(bag_info: &BagInfo, base_dir: P) -> Result<()> {
    write_tag_file(bag_info.as_ref(), base_dir.as_ref().join(BAG_INFO_TXT))
}

/// Reads a bag declaration out of the specified `base_dir`
pub fn read_bag_declaration<P: AsRef<Path>>(base_dir: P) -> Result<BagDeclaration> {
    let bagit_file = base_dir.as_ref().join(BAGIT_TXT);
    let tags = read_tag_file(&bagit_file)?;
    tags.try_into()
}

/// Reads bag info out of the specified `base_dir`
pub fn read_bag_info<P: AsRef<Path>>(base_dir: P) -> Result<BagInfo> {
    let bagit_file = base_dir.as_ref().join(BAG_INFO_TXT);
    let tags = read_tag_file(&bagit_file)?;
    Ok(tags.into())
}

impl BagDeclaration {
    pub fn new() -> Self {
        Self {
            version: BAGIT_DEFAULT_VERSION,
            // TODO encoding
            encoding: UTF_8.into(),
        }
    }

    pub fn with_values<S: AsRef<str>>(version: BagItVersion, encoding: S) -> Result<Self> {
        let encoding = encoding.as_ref();

        if BAGIT_1_0 != version {
            return Err(UnsupportedVersion { version });
        }

        if UTF_8 != encoding {
            return Err(UnsupportedEncoding {
                encoding: encoding.into(),
            });
        }

        Ok(Self {
            version,
            encoding: encoding.into(),
        })
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

impl TryFrom<TagList> for BagDeclaration {
    type Error = Error;

    fn try_from(tags: TagList) -> std::result::Result<Self, Self::Error> {
        let version_tag = tags
            .get_tag(LABEL_BAGIT_VERSION)
            .ok_or_else(|| MissingTag {
                tag: LABEL_BAGIT_VERSION.to_string(),
            })?;
        let version = BagItVersion::try_from(&version_tag.value)?;

        let encoding_tag = tags
            .get_tag(LABEL_FILE_ENCODING)
            .ok_or_else(|| MissingTag {
                tag: LABEL_FILE_ENCODING.to_string(),
            })?;
        let encoding = &encoding_tag.value;

        BagDeclaration::with_values(version, encoding)
    }
}

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

    pub fn with_generated<D: AsRef<str>, O: AsRef<str>>(bagging_date: D, payload_oxum: O) -> Self {
        let mut info = Self::with_capacity(2);
        info.add_bagging_date(bagging_date);
        info.add_payload_oxum(payload_oxum);
        info
    }

    pub fn with_tags(tags: TagList) -> Self {
        Self { tags }
    }

    pub fn add_bagging_date<S: AsRef<str>>(&mut self, value: S) {
        self.tags.remove_tags(LABEL_BAGGING_DATE);
        self.tags.add_tag(LABEL_BAGGING_DATE, value);
    }

    pub fn add_payload_oxum<S: AsRef<str>>(&mut self, value: S) {
        self.tags.remove_tags(LABEL_PAYLOAD_OXUM);
        self.tags.add_tag(LABEL_PAYLOAD_OXUM, value);
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

impl AsRef<TagList> for BagInfo {
    fn as_ref(&self) -> &TagList {
        &self.tags
    }
}

impl Tag {
    // TODO validate label does not contain `:`
    // TODO validate values
    pub fn new<L: AsRef<str>, V: AsRef<str>>(label: L, value: V) -> Self {
        Self {
            label: label.as_ref().into(),
            value: value.as_ref().into(),
        }
    }
}

impl TagList {
    pub fn new() -> Self {
        Self { tags: Vec::new() }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            tags: Vec::with_capacity(capacity),
        }
    }

    /// Returns all of the tags with the provided label. It uses a case insensitive match.
    pub fn get_tags<S: AsRef<str>>(&self, label: S) -> Vec<&Tag> {
        let label = label.as_ref();
        self.tags
            .iter()
            .filter(|tag| tag.label.eq_ignore_ascii_case(label))
            .collect()
    }

    /// Returns the first tag with the provided label. It uses a case insensitive match.
    pub fn get_tag<S: AsRef<str>>(&self, label: S) -> Option<&Tag> {
        let label = label.as_ref();
        self.tags
            .iter()
            .find(|tag| tag.label.eq_ignore_ascii_case(label))
    }

    pub fn add(&mut self, tag: Tag) {
        self.tags.push(tag);
    }

    pub fn add_tag<L: AsRef<str>, V: AsRef<str>>(&mut self, label: L, value: V) {
        self.tags.push(Tag::new(label, value));
    }

    /// Removes all of the tags with the provided label. It uses a case insensitive match.
    pub fn remove_tags<S: AsRef<str>>(&mut self, label: S) {
        let label = label.as_ref();
        self.tags.retain(|e| !e.label.eq_ignore_ascii_case(label));
    }
}

impl Default for TagList {
    fn default() -> Self {
        Self::new()
    }
}

impl IntoIterator for TagList {
    type Item = Tag;
    type IntoIter = IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.tags.into_iter()
    }
}

impl<'a> IntoIterator for &'a TagList {
    type Item = &'a Tag;
    type IntoIter = Iter<'a, Tag>;

    fn into_iter(self) -> Self::IntoIter {
        self.tags.iter()
    }
}

/// Writes a tag file to the specified destination
fn write_tag_file<P: AsRef<Path>>(tags: &TagList, destination: P) -> Result<()> {
    let destination = destination.as_ref();
    info!("Writing tag file {}", destination.display());

    let mut writer =
        BufWriter::new(File::create(destination).context(IoCreateSnafu { path: destination })?);

    for tag in tags {
        // TODO handle multi-line tags
        writeln!(writer, "{}: {}", tag.label, tag.value)
            .context(IoWriteSnafu { path: destination })?;
    }

    Ok(())
}

fn read_tag_file<P: AsRef<Path>>(path: P) -> Result<TagList> {
    let path = path.as_ref();
    let mut reader = BufReader::new(File::open(path).context(IoReadSnafu { path })?);

    let mut tags = TagList::new();
    let mut line = String::new();

    loop {
        // TODO this only works for UTF-8
        let read = reader.read_line(&mut line).context(IoReadSnafu { path })?;

        if read == 0 {
            break;
        }

        // TODO incomplete: must account for multi-line tags
        tags.add(parse_tag_line(&line)?);

        line.clear();
    }

    Ok(tags)
}

fn parse_tag_line<S: AsRef<str>>(line: S) -> Result<Tag> {
    let line = line.as_ref();

    if let Some((label, value)) = line.split_once(':') {
        debug!("Tag [`{label}`:`{value}`]");
        let char1 = value.chars().next();

        if char1.is_none() || !(char1.unwrap() == ' ' || char1.unwrap() == '\t') {
            Err(InvalidTagLine {
                line: line.into(),
                details: "value part must start with one whitespace character".to_string(),
            })
        } else {
            // TODO does this work for CRLF as well?
            let trim_value = if value.ends_with('\n') {
                &value[1..value.len() - 1]
            } else {
                &value[1..]
            };

            debug!("Tag [`{label}`:`{trim_value}`]");
            Ok(Tag::new(label, trim_value))
        }
    } else {
        Err(InvalidTagLine {
            line: line.into(),
            details: "missing colon".to_string(),
        })
    }
}
