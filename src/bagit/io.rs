use crate::bagit::consts::*;
use crate::bagit::error::*;
use crate::bagit::Error::IoGeneral;
use snafu::ResultExt;
use std::io::Read;

/// Iteratively reads lines. Lines can be terminated by CR, LF, or CRLF.
pub struct LineReader<R: Read> {
    reader: R,
    buf: [u8; BUF_SIZE],
    position: usize,
    read: usize,
    end: bool,
}

/// Iteratively reads BagIt tag lines. Tag lines can be terminated by CR, LF, or CRLF. Lines
/// that have any number of leading spaces or tabs are considered to be part of the previous line.
/// All connected lines are joined by stripping the leading whitespace and inserting a single space.
pub struct TagLineReader<R: Read> {
    reader: LineReader<R>,
    next: Option<String>,
}

pub fn is_space_or_tab(c: char) -> bool {
    c == SPACE || c == TAB
}

impl<R: Read> LineReader<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            buf: [0; BUF_SIZE],
            position: 0,
            read: 0,
            end: false,
        }
    }
}

impl<R: Read> Iterator for LineReader<R> {
    type Item = Result<String>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.end {
            return None;
        }

        let mut line = Vec::new();

        loop {
            if self.position >= self.read {
                match self.reader.read(&mut self.buf) {
                    Ok(read) => {
                        if read == 0 {
                            self.end = true;
                        } else {
                            self.read = read;
                            self.position = 0;
                        }
                    }
                    Err(e) => return Some(Err(IoGeneral { source: e })),
                }
            }

            if self.end {
                return if line.is_empty() {
                    None
                } else {
                    Some(bytes_to_string(line))
                };
            }

            let mut seen_cr = false;
            let mut found_end = false;

            for i in self.position..self.read {
                let b = self.buf[i];

                if seen_cr && b != LF_B {
                    found_end = true;
                    self.position = i;
                    break;
                } else if b == CR_B {
                    seen_cr = true;
                } else if b == LF_B {
                    found_end = true;
                    self.position = i + 1;
                    break;
                } else {
                    line.push(b);
                }
            }

            // Read the whole buffer but didn't find the end of the line, try again
            if !found_end {
                self.position = 0;
                self.read = 0;
                continue;
            }

            return Some(bytes_to_string(line));
        }
    }
}

impl<R: Read> TagLineReader<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader: LineReader::new(reader),
            next: None,
        }
    }
}

impl<R: Read> Iterator for TagLineReader<R> {
    type Item = Result<String>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut current = self.next.take();

        loop {
            match self.reader.next() {
                Some(Err(e)) => return Some(Err(e)),
                None => {
                    return if current.is_some() {
                        Some(Ok(current.take().unwrap()))
                    } else {
                        None
                    };
                }
                Some(Ok(read)) => {
                    if current.is_some() && read.starts_with(is_space_or_tab) {
                        let current = current.as_mut().unwrap();
                        current.push(SPACE);
                        current.push_str(read.trim_start_matches(is_space_or_tab));
                    } else if current.is_some() {
                        self.next = Some(read);
                        return Some(Ok(current.take().unwrap()));
                    } else {
                        current = Some(read);
                    }
                }
            }
        }
    }
}

fn bytes_to_string(bytes: Vec<u8>) -> Result<String> {
    String::from_utf8(bytes).context(InvalidStringSnafu {})
}

#[cfg(test)]
mod tests {
    use crate::bagit::io::{LineReader, TagLineReader};
    use std::io::BufReader;

    #[test]
    fn read_lines_with_different_endings_no_endline() {
        let input = "line 1\rline 2\r\rline 3\r\nline 4\nline 5\rline 6\r\nline 7\n\rline 8";
        let reader = LineReader::new(BufReader::new(input.as_bytes()));

        let lines: Vec<String> = reader.flatten().collect();

        assert_eq!(
            vec![
                "line 1", "line 2", "", "line 3", "line 4", "line 5", "line 6", "line 7", "",
                "line 8"
            ],
            lines
        );
    }

    #[test]
    fn read_lines_with_different_endings() {
        let input = "\r\nline 1\rline 2\r\nline 3\n";
        let reader = LineReader::new(BufReader::new(input.as_bytes()));

        let lines: Vec<String> = reader.flatten().collect();

        assert_eq!(vec!["", "line 1", "line 2", "line 3"], lines);
    }

    #[test]
    fn read_multi_line_tags() {
        let input =
            "tag-1: normal tag\ntag-2: 1\r 2\n\t3\r\ntag-3:\t4\n   5\n  \n \t 6\ntag-4: end";
        let reader = TagLineReader::new(BufReader::new(input.as_bytes()));

        let lines: Vec<String> = reader.flatten().collect();

        assert_eq!(
            vec![
                "tag-1: normal tag",
                "tag-2: 1 2 3",
                "tag-3:\t4 5  6",
                "tag-4: end"
            ],
            lines
        );
    }
}
