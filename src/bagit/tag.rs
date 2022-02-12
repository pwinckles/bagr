use log::info;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::slice::Iter;
use std::vec::IntoIter;

use snafu::ResultExt;

use crate::bagit::error::*;

#[derive(Debug)]
pub struct Tag {
    label: String,
    value: String,
}

#[derive(Debug)]
pub struct TagList {
    tags: Vec<Tag>,
}

pub fn write_tag_file<P: AsRef<Path>>(tags: &TagList, destination: P) -> Result<()> {
    let destination = destination.as_ref();
    info!("Writing tag file {}", destination.display());

    let mut writer =
        BufWriter::new(File::create(destination).context(IoCreateSnafu { path: destination })?);

    for tag in tags {
        // TODO temp
        writeln!(writer, "{}: {}", tag.label, tag.value)
            .context(IoWriteSnafu { path: destination })?;
    }

    Ok(())
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

    pub fn add_tag<L: AsRef<str>, V: AsRef<str>>(&mut self, label: L, value: V) {
        self.tags.push(Tag::new(label, value));
    }

    pub fn remove_tags<S: AsRef<str>>(&mut self, label: S) {
        let label = label.as_ref();
        self.tags.retain(|e| e.label != label);
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
