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

// TODO reader and writer separate?
pub struct TagFileWriter {
    // TODO base_dir?
// TODO encoding?
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
}

impl Default for TagList {
    fn default() -> Self {
        TagList::new()
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

impl TagFileWriter {
    pub fn new() -> Self {
        Self {}
    }

    // TODO is this the right data structure?
    pub fn write<P: AsRef<Path>>(&self, tags: &TagList, destination: P) -> Result<()> {
        // TODO info log
        let mut writer = BufWriter::new(File::create(&destination).context(IoCreateSnafu {
            path: destination.as_ref().to_path_buf(),
        })?);

        for tag in tags {
            // TODO temp
            writeln!(writer, "{}: {}", tag.label, tag.value).context(IoWriteSnafu {
                path: destination.as_ref().to_path_buf(),
            })?;
        }

        // TODO should there be a blank line at the end of this file?

        Ok(())
    }
}
