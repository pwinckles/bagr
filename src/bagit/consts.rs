use crate::bagit::bag::BagItVersion;
use crate::bagit::DigestAlgorithm;
use once_cell::sync::Lazy;
use regex::Regex;

pub static PAYLOAD_MANIFEST_MATCHER: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^manifest-([[:alnum:]]+)\.txt$").unwrap());
pub static TAG_MANIFEST_MATCHER: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^tagmanifest-([[:alnum:]]+)\.txt$").unwrap());

pub const BAGR_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const BAGR_SRC_URL: &str = "https://github.com/pwinckles/bagr";

pub const BAGIT_1_0: BagItVersion = BagItVersion::new(1, 0);
pub const BAGIT_DEFAULT_VERSION: BagItVersion = BAGIT_1_0;

pub const DEFAULT_ALGORITHM: DigestAlgorithm = DigestAlgorithm::Sha512;

pub const UTF_8: &str = "UTF-8";

pub const CR: char = '\r';
pub const LF: char = '\n';
pub const TAB: char = '\t';
pub const SPACE: char = ' ';
pub const CR_B: u8 = b'\r';
pub const LF_B: u8 = b'\n';
pub const BUF_SIZE: usize = 8 * 1024;

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
pub const LABEL_SOFTWARE_AGENT: &str = "Bag-Software-Agent";
pub const LABEL_SOURCE_ORGANIZATION: &str = "Source-Organization";
pub const LABEL_ORGANIZATION_ADDRESS: &str = "Organization-Address";
pub const LABEL_CONTACT_NAME: &str = "Contact-Name";
pub const LABEL_CONTACT_PHONE: &str = "Contact-Phone";
pub const LABEL_CONTACT_EMAIL: &str = "Contact-Email";
pub const LABEL_EXTERNAL_DESCRIPTION: &str = "External-Description";
pub const LABEL_EXTERNAL_IDENTIFIER: &str = "External-Identifier";
pub const LABEL_BAG_SIZE: &str = "Bag-Size";
pub const LABEL_BAG_GROUP_IDENTIFIER: &str = "Bag-Group-Identifier";
pub const LABEL_BAG_COUNT: &str = "Bag-Count";
pub const LABEL_INTERNAL_SENDER_IDENTIFIER: &str = "Internal-Sender-Identifier";
pub const LABEL_INTERNAL_SENDER_DESCRIPTION: &str = "Internal-Sender-Description";
pub const LABEL_BAGIT_PROFILE_IDENTIFIER: &str = "BagIt-Profile-Identifier";

/// Lookup table that indicates if a reserved bag-info label is repeatable. All label names are
/// lowercased here.
pub const LABEL_REPEATABLE: [(&str, bool); 16] = [
    ("bagging-date", false),
    ("payload-oxum", false),
    ("bag-software-agent", false),
    ("source-organization", true),
    ("organization-address", true),
    ("contact-name", true),
    ("contact-phone", true),
    ("contact-email", true),
    ("external-description", true),
    ("external-identifier", true),
    ("bag-size", false),
    ("bag-group-identifier", false),
    ("bag-count", false),
    ("internal-sender-identifier", true),
    ("internal-sender-description", true),
    ("bagit-profile-identifier", true),
];
