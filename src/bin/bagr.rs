use std::path::PathBuf;
use std::process::exit;

use bagr::bagit::{create_bag, open_bag, BagInfo, DigestAlgorithm};
use clap::AppSettings::UseLongFormatForHelpSubcommand;
use clap::{Args, Parser, Subcommand};
use log::{error, info, LevelFilter};

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

    /// Disable all output styling
    #[clap(short = 'S', long)]
    pub no_styles: bool,

    /// Subcommand to execute
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
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

    /// Set the Bagging-Date tag
    ///
    /// Defaults to the current date. Should be in YYYY-MM-DD format.
    #[clap(long, value_name = "YYYY-MM-DD")]
    pub bagging_date: Option<String>,
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
            let algorithms = &[DigestAlgorithm::Md5, DigestAlgorithm::Sha256];

            let mut bag_info = BagInfo::new();

            if let Some(date) = sub_args.bagging_date {
                // TODO error
                bag_info.add_bagging_date(date).unwrap();
            }

            if let Err(e) = create_bag(
                defaulted_path(sub_args.source),
                defaulted_path(sub_args.destination),
                bag_info,
                algorithms,
            ) {
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

fn defaulted_path(path: Option<PathBuf>) -> PathBuf {
    path.unwrap_or_else(|| PathBuf::from("."))
}
