pub use crate::bagit::bag::{create_bag, open_bag, Bag, BagItVersion};
pub use crate::bagit::digest::DigestAlgorithm;
pub use crate::bagit::error::*;
pub use crate::bagit::tag::{BagDeclaration, BagInfo};

mod bag;
mod consts;
mod digest;
mod encoding;
mod error;
mod io;
mod tag;
