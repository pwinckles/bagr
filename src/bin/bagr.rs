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

    /// Digest algorithms to use when creating manifest files.
    ///
    /// By default, the same algorithms are used as were used to compute the existing manifests.
    /// If algorithms are specified here, then only the specified algorithms will be used, and
    /// the algorithms used by the existing manifests will be ignored.
    #[clap(
        arg_enum,
        short = 'a',
        long,
        value_name = "ALGORITHM",
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

    match args.command {
        Command::Bag(cmd) => {
            if let Err(e) = exec_bag(cmd) {
                error!("Failed to create bag: {}", e);
                exit(1);
            }
        }
        Command::Rebag(cmd) => {
            if let Err(e) = exec_rebag(cmd) {
                error!("Failed to rebag: {}", e);
                exit(1);
            }
        }
    }
}

fn exec_bag(cmd: BagCmd) -> Result<Bag> {
    let mut bag_info = BagInfo::new();

    if let Some(date) = cmd.bagging_date {
        bag_info.add_bagging_date(date)?;
    }
    if let Some(agent) = cmd.software_agent {
        bag_info.add_software_agent(agent)?;
    }
    if let Some(group_id) = cmd.bag_group_identifier {
        bag_info.add_bag_group_identifier(group_id)?;
    }
    if let Some(count) = cmd.bag_count {
        bag_info.add_bag_count(count)?;
    }
    if let Some(size) = cmd.bag_size {
        bag_info.add_bag_size(size)?;
    }

    for org in cmd.source_organization {
        bag_info.add_source_organization(org)?;
    }
    for address in cmd.organization_address {
        bag_info.add_organization_address(address)?;
    }
    for name in cmd.contact_name {
        bag_info.add_contact_name(name)?;
    }
    for phone in cmd.contact_phone {
        bag_info.add_contact_phone(phone)?;
    }
    for email in cmd.contact_email {
        bag_info.add_contact_email(email)?;
    }
    for desc in cmd.external_description {
        bag_info.add_external_description(desc)?;
    }
    for id in cmd.external_identifier {
        bag_info.add_external_identifier(id)?;
    }
    for desc in cmd.internal_sender_description {
        bag_info.add_internal_sender_description(desc)?;
    }
    for id in cmd.internal_sender_identifier {
        bag_info.add_internal_sender_identifier(id)?;
    }

    for tag in cmd.tag {
        let split = tag.split_once(':').ok_or_else(|| InvalidTagLine {
            details: format!("Label and value must be separated by a ':'. Found: {}", tag),
        })?;
        bag_info.add_tag(split.0.trim(), split.1.trim())?;
    }

    create_bag(
        defaulted_path(cmd.source),
        defaulted_path(cmd.destination),
        bag_info,
        &map_algorithms(&cmd.digest_algorithm),
    )
}

fn exec_rebag(cmd: RebagCmd) -> Result<Bag> {
    let bag = open_bag(defaulted_path(cmd.bag_path))?;
    info!("Opened bag: {:?}", bag);

    // TODO add option for not recalculating payload manifests

    bag.update()
        .with_bagging_date(cmd.bagging_date)
        .with_software_agent(cmd.software_agent)
        .with_algorithms(&map_algorithms(&cmd.digest_algorithm))
        .finalize()
}

fn defaulted_path(path: Option<PathBuf>) -> PathBuf {
    path.unwrap_or_else(|| PathBuf::from("."))
}

fn map_algorithms(algorithms: &[DigestAlgorithm]) -> Vec<BagItDigestAlgorithm> {
    algorithms
        .iter()
        .map(|e| (*e).into())
        .collect::<Vec<BagItDigestAlgorithm>>()
}
