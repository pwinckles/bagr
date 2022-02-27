use std::io::ErrorKind;
use std::path::Path;

use crate::bagit::consts::*;
use crate::bagit::error::*;
use crate::bagit::tag::read_bag_declaration;
use crate::bagit::BagDeclaration;
use crate::bagit::Error::*;

#[derive(Debug)]
pub struct ValidationResult {
    verdict: ValidationVerdict,
    issues: Vec<ValidationIssue>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ValidationVerdict {
    Valid,
    Complete,
    Invalid,
}

#[derive(Debug)]
pub struct ValidationIssue {
    level: IssueLevel,
    message: String,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum IssueLevel {
    Error,
    Warn,
}

/// Validates the bag at the specified path. If `integrity_check` is `true` then the checksums of
/// all of the files in the bag will be verified. Otherwise, the bag is only evaluated based on
/// whether it is complete.
pub fn validate_bag<P: AsRef<Path>>(
    base_dir: P,
    integrity_check: bool,
) -> Result<ValidationResult> {
    // TODO from the rfc
    // A _complete_ bag MUST meet the following requirements:
    //     1.  Every required element MUST be present (see Section 2.1).
    //         a. bagit.txt
    //            1. BagIt-Version
    //            2. Tag-File-Character-Encoding
    //         b. data dir
    //         c. 1+ payload manifest
    //            1. valid algorithm
    //            2. contain every file in payload
    //            3. not reference files outside of payload
    //            4. not reference directories
    //            5. valid line format
    //            6. MAY start with ./
    //     2.  Every file listed in every tag manifest MUST be present.
    //         a. not reference files outside of bag
    //         b. not reference payload files
    //         c. not reference tag manifests
    //         d. MUST reference payload manifests
    //         e. SHOULD reference other tag files
    //         f. all tag manifests SHOULD list same files
    //         g. all algorithms SHOULD be the same as payload manifests
    //     3.  Every file listed in every payload manifest MUST be present.
    //     4.  For BagIt 1.0, every payload file MUST be listed in every payload
    //         manifest.  Note that older versions of BagIt allowed payload
    //         files to be listed in just one of the manifests.
    //     5.  Every element present MUST conform to BagIt 1.0.
    //         a. bag-info.txt
    //            1. valid lines
    //            2. warnings based on reserved labels
    //         b. fetch.txt
    //            1. valid lines
    //            2. files listed must be in payload manifest
    //            3. not reference tag files
    //
    // A _valid_ bag MUST meet the following requirements:
    //     1.  The bag MUST be _complete_.
    //     2.  Every checksum in every payload manifest and tag manifest has
    //         been successfully verified against the contents of the
    //         corresponding file.

    let base_dir = base_dir.as_ref();
    let mut result = if integrity_check {
        ValidationResult::new_valid()
    } else {
        ValidationResult::new_complete()
    };

    let declaration = read_and_validate_declaration(base_dir, &mut result)?;

    // cannot continue without a valid declaration because we wouldn't know
    // what version to validate against
    if declaration.is_none() {
        return Ok(result);
    }

    let _declaration = declaration.unwrap();

    Ok(result)
}

impl ValidationResult {
    pub fn new_valid() -> Self {
        Self {
            verdict: ValidationVerdict::Valid,
            issues: Vec::new(),
        }
    }

    pub fn new_complete() -> Self {
        Self {
            verdict: ValidationVerdict::Complete,
            issues: Vec::new(),
        }
    }

    pub fn invalid(&mut self) {
        self.verdict = ValidationVerdict::Invalid;
    }

    pub fn error<S: AsRef<str>>(&mut self, message: S) {
        self.issues.push(ValidationIssue::error(message));
    }

    pub fn warn<S: AsRef<str>>(&mut self, message: S) {
        self.issues.push(ValidationIssue::warn(message));
    }
}

impl ValidationIssue {
    pub fn error<S: AsRef<str>>(message: S) -> Self {
        Self {
            level: IssueLevel::Error,
            message: message.as_ref().into(),
        }
    }

    pub fn warn<S: AsRef<str>>(message: S) -> Self {
        Self {
            level: IssueLevel::Warn,
            message: message.as_ref().into(),
        }
    }
}

fn read_and_validate_declaration<P: AsRef<Path>>(
    base_dir: P,
    result: &mut ValidationResult,
) -> Result<Option<BagDeclaration>> {
    fn add_error(result: &mut ValidationResult, message: String)
        -> Result<Option<BagDeclaration>> {
        result.invalid();
        result.error(message);
        Ok(None)
    }

    match read_bag_declaration(base_dir) {
        Ok(declaration) => Ok(Some(declaration)),
        Err(IoRead { source, path }) => match source.kind() {
            ErrorKind::NotFound => add_error(result, format!("{} does not exist", BAGIT_TXT)),
            ErrorKind::PermissionDenied => {
                add_error(result, format!("{} cannot be read", BAGIT_TXT))
            }
            _ => Err(IoRead { source, path }),
        },
        Err(InvalidTagLineWithRef { details, num, .. }) => add_error(
            result,
            format!("Tag {} in {} is invalid: {}", num, BAGIT_TXT, details),
        ),
        Err(MissingTag { tag }) => add_error(
            result,
            format!("{} is missing required tag '{}'", BAGIT_TXT, tag),
        ),
        Err(InvalidBagItVersion { value }) => add_error(
            result,
            format!(
                "{} contains an invalid {}: {}",
                BAGIT_TXT, LABEL_BAGIT_VERSION, value
            ),
        ),
        Err(UnsupportedEncoding { encoding }) => add_error(
            result,
            format!(
                "{} contains an invalid {}: {}",
                BAGIT_TXT, LABEL_FILE_ENCODING, encoding
            ),
        ),
        Err(e) => Err(e),
    }
}
