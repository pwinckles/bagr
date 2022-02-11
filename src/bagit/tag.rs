use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use snafu::ResultExt;

use crate::bagit::error::*;

#[derive(Debug)]
pub struct Tag {
    label: String,
    values: Vec<String>,
}

// TODO reader and writer separate?
pub struct TagFileWriter {
    // TODO base_dir?
    // TODO encoding?
}

impl Tag {
    // TODO validate label does not contain `:` here
    pub fn new<L: AsRef<str>>(label: L) -> Self {
        Self {
            label: label.as_ref().into(),
            values: Vec::new(),
        }
    }

    pub fn with_value<L: AsRef<str>, V: AsRef<str>>(label: L, value: V) -> Self {
        Self {
            label: label.as_ref().into(),
            values: vec![value.as_ref().into()],
        }
    }

    // TODO validate values here
    pub fn add_value<V: AsRef<str>>(&mut self, value: V) {
        self.values.push(value.as_ref().into());
    }
}

impl TagFileWriter {
    pub fn new() -> Self {
        Self {}
    }

    // TODO is this the right data structure?
    pub fn write<P: AsRef<Path>>(&self, tags: &[Tag], destination: P) -> Result<()> {
        // TODO info log
        let mut writer = BufWriter::new(File::create(&destination).context(IoCreateSnafu {
            path: destination.as_ref().to_path_buf(),
        })?);

        for tag in tags {
            for value in &tag.values {
                // TODO temp
                writeln!(writer, "{}: {}", tag.label, value).context(IoWriteSnafu {
                    path: destination.as_ref().to_path_buf(),
                })?;
            }
        }

        // TODO should there be a blank line at the end of this file?

        Ok(())
    }

}
