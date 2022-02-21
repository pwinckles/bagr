use std::path::PathBuf;
use std::process::exit;

use clap::AppSettings::UseLongFormatForHelpSubcommand;
use clap::{ArgEnum, Args, Parser, Subcommand};
use log::{error, info, LevelFilter};

use bagr::bagit::Error::InvalidTagLine;
use bagr::bagit::{
    create_bag, open_bag, Bag, BagInfo, DigestAlgorithm as BagItDigestAlgorithm, Result,
};

// TODO expand docs

/// A CLI for interacting with BagIt bags
#[derive(Debug, Parser)]
#[clap(name = "bagr", author = "Peter Winckles <pwinckles@pm.me>", version)]
#[clap(setting(UseLongFormatForHelpSubcommand))]
pub struct BagrArgs {
    /// Suppress error messages and other command specific logging
    #[clap(short, long)]
    pub quiet: bool,

    /// Increase log level
    #[clap(short = 'V', long)]
    pub verbose: bool,

    // TODO this might not be needed
    /// Disable all output styling
    #[clap(short = 'S', long)]
    pub no_styles: bool,

    /// Subcommand to execute
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum Command {
    #[clap(name = "bag")]
    Bag(BagCmd),
    #[clap(name = "rebag")]
    Rebag(RebagCmd),
}

/// Create a new bag
#[derive(Args, Debug)]
pub struct BagCmd {
    /// Absolute or relative path to the new bag's base directory
    ///
    /// By default, this is the current directory.
    #[clap(short, long, value_name = "DST_DIR")]
    pub destination: Option<PathBuf>,

    /// Absolute or relative path to the directory containing the files to add to the bag
    ///
    /// Specify this option to create a bag by copying files from a directory into a bag in
    /// a different directory. By default, bags are created in place.
    #[clap(short, long, value_name = "SRC_DIR")]
    pub source: Option<PathBuf>,

    /// Digest algorithms to use when creating manifest files.
    ///
    /// A manifest is created for each algorithm that's specified
    #[clap(
        arg_enum,
        short = 'a',
        long,
        value_name = "ALGORITHM",
        default_value = "sha512",
        ignore_case = true,
        multiple_occurrences = true
    )]
    pub digest_algorithm: Vec<DigestAlgorithm>,

    /// Value of the Bagging-Date tag in bag-info.txt
    ///
    /// Defaults to the current date. Should be in YYYY-MM-DD format.
    #[clap(long, value_name = "YYYY-MM-DD")]
    pub bagging_date: Option<String>,

    /// Value of the Bag-Software-Agent tag in bag-info.txt
    ///
    /// Defaults to this bagr version
    #[clap(long, value_name = "AGENT")]
    pub software_agent: Option<String>,

    /// Value of the Bagging-Size tag in bag-info.txt
    #[clap(long, value_name = "SIZE")]
    pub bag_size: Option<String>,

    /// Value of the Bag-Group-Identifier tag in bag-info.txt
    #[clap(long, value_name = "BAG_GROUP_ID")]
    pub bag_group_identifier: Option<String>,

    /// Value of the Bag-Count tag in bag-info.txt. Should be in the form "N of T".
    #[clap(long, value_name = "BAG_COUNT")]
    pub bag_count: Option<String>,

    /// Value of the Source-Organization tag in bag-info.txt. Maybe repeated.
    #[clap(long, value_name = "ORG", multiple_occurrences = true)]
    pub source_organization: Vec<String>,

    /// Value of the Organization-Address tag in bag-info.txt. Maybe repeated.
    #[clap(long, value_name = "ORG_ADDR", multiple_occurrences = true)]
    pub organization_address: Vec<String>,

    /// Value of the Contact-Name tag in bag-info.txt. Maybe repeated.
    #[clap(long, value_name = "CONTACT_NAME", multiple_occurrences = true)]
    pub contact_name: Vec<String>,

    /// Value of the Contact-Phone tag in bag-info.txt. Maybe repeated.
    #[clap(long, value_name = "CONTACT_PHONE", multiple_occurrences = true)]
    pub contact_phone: Vec<String>,

    /// Value of the Contact-Email tag in bag-info.txt. Maybe repeated.
    #[clap(long, value_name = "CONTACT_EMAIL", multiple_occurrences = true)]
    pub contact_email: Vec<String>,

    /// Value of the External-Description tag in bag-info.txt. Maybe repeated.
    #[clap(long, value_name = "EXT_DESC", multiple_occurrences = true)]
    pub external_description: Vec<String>,

    /// Value of the External-Identifier tag in bag-info.txt. Maybe repeated.
    #[clap(long, value_name = "EXT_ID", multiple_occurrences = true)]
    pub external_identifier: Vec<String>,

    /// Value of the Internal-Sender-Identifier tag in bag-info.txt. Maybe repeated.
    #[clap(long, value_name = "INT_SENDER_ID", multiple_occurrences = true)]
    pub internal_sender_identifier: Vec<String>,

    /// Value of the Internal-Sender-Description tag in bag-info.txt. Maybe repeated.
    #[clap(long, value_name = "INT_SENDER_DESC", multiple_occurrences = true)]
    pub internal_sender_description: Vec<String>,

    /// A custom tag to add to bag-info.txt. Tags must be formatted as LABEL:VALUE
    #[clap(short, long, value_name = "LABEL:VALUE", multiple_occurrences = true)]
    pub tag: Vec<String>,
}

/// Update BagIt manifests to match the current state on disk
#[derive(Args, Debug)]
pub struct RebagCmd {
    /// Absolute or relative path to the bag's base directory
    ///
    /// By default, this is the current directory.
    #[clap(short, long, value_name = "BAG_PATH")]
    pub bag_path: Option<PathBuf>,
}

#[derive(ArgEnum, Debug, Clone, Copy)]
pub enum DigestAlgorithm {
    Md5,
    Sha1,
    Sha256,
    Sha512,
    Blake2b256,
    Blake2b512,
}

impl From<DigestAlgorithm> for BagItDigestAlgorithm {
    fn from(algorithm: DigestAlgorithm) -> Self {
        match algorithm {
            DigestAlgorithm::Md5 => BagItDigestAlgorithm::Md5,
            DigestAlgorithm::Sha1 => BagItDigestAlgorithm::Sha1,
            DigestAlgorithm::Sha256 => BagItDigestAlgorithm::Sha256,
            DigestAlgorithm::Sha512 => BagItDigestAlgorithm::Sha512,
            DigestAlgorithm::Blake2b256 => BagItDigestAlgorithm::Blake2b256,
            DigestAlgorithm::Blake2b512 => BagItDigestAlgorithm::Blake2b512,
        }
    }
}

fn main() {
    let mut args = BagrArgs::parse();

    let log_level = if args.quiet {
        LevelFilter::Off
    } else if args.verbose {
        LevelFilter::Info
    } else {
        LevelFilter::Warn
    };

    env_logger::builder()
        .filter_level(log_level)
        .format_timestamp(None)
        .format_module_path(false)
        .format_target(false)
        .init();

    // If the output is being piped then we should disable styling
    if atty::isnt(atty::Stream::Stdout) {
        args.no_styles = true;
    }

    // TODO
    match args.command {
        Command::Bag(sub_args) => {
            if let Err(e) = bag_cmd(sub_args) {
                error!("Failed to create bag: {}", e);
                exit(1);
            }
        }
        Command::Rebag(sub_args) => match open_bag(defaulted_path(sub_args.bag_path)) {
            Ok(bag) => {
                info!("Opened bag: {:?}", bag);

                if let Err(e) = bag.update().finalize() {
                    error!("Failed to rebag: {}", e);
                    exit(1);
                }
            }
            Err(e) => {
                error!("Failed to rebag: {}", e);
                exit(1);
            }
        },
    }
}

fn bag_cmd(sub_args: BagCmd) -> Result<Bag> {
    let mut bag_info = BagInfo::new();

    if let Some(date) = sub_args.bagging_date {
        bag_info.add_bagging_date(date)?;
    }
    if let Some(agent) = sub_args.software_agent {
        bag_info.add_software_agent(agent)?;
    }
    if let Some(group_id) = sub_args.bag_group_identifier {
        bag_info.add_bag_group_identifier(group_id)?;
    }
    if let Some(count) = sub_args.bag_count {
        bag_info.add_bag_count(count)?;
    }
    if let Some(size) = sub_args.bag_size {
        bag_info.add_bag_size(size)?;
    }

    for org in sub_args.source_organization {
        bag_info.add_source_organization(org)?;
    }
    for address in sub_args.organization_address {
        bag_info.add_organization_address(address)?;
    }
    for name in sub_args.contact_name {
        bag_info.add_contact_name(name)?;
    }
    for phone in sub_args.contact_phone {
        bag_info.add_contact_phone(phone)?;
    }
    for email in sub_args.contact_email {
        bag_info.add_contact_email(email)?;
    }
    for desc in sub_args.external_description {
        bag_info.add_external_description(desc)?;
    }
    for id in sub_args.external_identifier {
        bag_info.add_external_identifier(id)?;
    }
    for desc in sub_args.internal_sender_description {
        bag_info.add_internal_sender_description(desc)?;
    }
    for id in sub_args.internal_sender_identifier {
        bag_info.add_internal_sender_identifier(id)?;
    }

    for tag in sub_args.tag {
        let split = tag.split_once(':').ok_or_else(|| InvalidTagLine {
            details: format!("Label and value must be separated by a ':'. Found: {}", tag),
        })?;
        bag_info.add_tag(split.0.trim(), split.1.trim())?;
    }

    // TODO test for invalid custom tags

    create_bag(
        defaulted_path(sub_args.source),
        defaulted_path(sub_args.destination),
        bag_info,
        &sub_args
            .digest_algorithm
            .into_iter()
            .map(|e| e.into())
            .collect::<Vec<BagItDigestAlgorithm>>(),
    )
}

fn defaulted_path(path: Option<PathBuf>) -> PathBuf {
    path.unwrap_or_else(|| PathBuf::from("."))
}
